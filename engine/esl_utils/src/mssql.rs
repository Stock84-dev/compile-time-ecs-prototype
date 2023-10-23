use core::ops::RangeInclusive;
use std::{
    io::SeekFrom,
    ops::Range,
    path::{PathBuf},
    time::Instant,
};

use async_stream::try_stream;
use bytemuck::{Pod, Zeroable};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use config::MssqlConfig;
use futures::{
    stream::{StreamExt},
    Future, TryFutureExt, TryStream, TryStreamExt,
};
use memmap2::{Advice, Mmap, MmapOptions};
use mouse::prelude::*;
use odbc::{
    create_environment_v3,
    odbc_safe::AutocommitOn,
    Connection,
    ResultSetState::{Data, NoData},
    SqlTimestamp, Statement,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    try_join,
};

/// Loads order flow data from filesystem. If it is not present, it will be downloaded from the
/// database. Provide a large batch size to improve download bandwidth. 512 MiB is a good starting
/// point.
pub async fn load_orderflow(
    config: &MssqlConfig,
    table: &str,
    range: Range<DateTime<Utc>>,
    batch_size: usize,
) -> Result<ArrayMap<OrderFlowTick>> {
    info!("Loading orderflow ticks...");
    debug!("request start {}, request end {}", range.start, range.end);
    if !config.cache_dir.exists_async().await {
        tokio::fs::create_dir_all(&config.cache_dir).await?;
    }
    // Avoiding set_extension because the table name may contain a dot.
    let path = config.cache_dir.join(format!("{table}.bin"));
    let timestamp_range = range.start.timestamp_nanos()..range.end.timestamp_nanos();
    let (file, _saved_range) =
        maybe_download(config, &path, table, timestamp_range.clone(), batch_size).await?;
    let span = tracing::info_span!("binary search");
    let _span = span.enter();
    let mmap = unsafe { Mmap::map(&file)? };
    mmap.advise(Advice::Random)?;
    #[cfg(target_os = "linux")]
    mmap.advise(Advice::NoHugePage)?;
    let data = mmap.as_ref();
    let ticks: &[OrderFlowTick] = bytemuck::cast_slice(data);
    let start_index = match ticks.binary_search_by_key(&timestamp_range.start, |x| x.timestamp_ns) {
        Ok(index) => index,
        Err(index) => index,
    };
    let end_index = match ticks.binary_search_by_key(&timestamp_range.end, |x| x.timestamp_ns) {
        Ok(index) => index,
        Err(index) => index,
    } - 1;
    drop(mmap);
    drop(_span);
    drop(span);
    let offset = start_index as u64 * OrderFlowTick::size() as u64;
    let len = end_index as u64 * OrderFlowTick::size() as u64 - offset;
    debug!("mapping {} GiB", len as f32 / 1024.0 / 1024.0 / 1024.0);
    let mmap = unsafe {
        MmapOptions::new()
            .offset(offset)
            .len(len as usize)
            .populate()
            .map(&file)?
    };
    mmap.advise(Advice::Sequential)?;

    Ok(ArrayMap {
        map: mmap,
        _t: _core::marker::PhantomData,
    })
}

pub struct ArrayMap<T: Pod> {
    map: Mmap,
    _t: std::marker::PhantomData<T>,
}

impl<T: Pod> ArrayMap<T> {
    pub fn slice(&self) -> &[T] {
        bytemuck::cast_slice(self.map.as_ref())
    }
}

#[instrument(skip(config))]
async fn download(
    config: &MssqlConfig,
    file: &mut File,
    table: &str,
    timestamp_range_ns: Range<i64>,
    include_start: bool,
    batch_size: usize,
) -> Result<()> {
    let connection = MssqlConnection::new(config)?;
    let stream = connection.orderflow_loader(
        table,
        timestamp_range_ns.start.into_date_time()..timestamp_range_ns.end.into_date_time(),
        include_start,
        batch_size / OrderFlowTick::size(),
    );

    let mut now = Instant::now();
    async_for! {
        async for batch in stream {
            let batch = batch?;
            let slice = bytemuck::cast_slice(&batch[..]);
            file.write_all(slice).await?;
            debug!(
                "DB cache write bandwidth {:.2} MiB/s",
                slice.len() as f32 / 1024.0 / 1024.0 / now.elapsed().as_secs_f32()
            );
            now = Instant::now();
        }
    }
    Ok(())
}

/// Returns the file provided with unspecified cursor position.
#[instrument(skip(config))]
async fn maybe_download(
    config: &MssqlConfig,
    path: &PathBuf,
    table: &str,
    timestamp_range_ns: Range<i64>,
    batch_size: usize,
) -> Result<(File, RangeInclusive<i64>)> {
    let mut file = open_rwc_async(path).await?;
    let file_size = file.metadata().await?.len();
    let mut downloaded = false;
    if (file_size / OrderFlowTick::size() as u64) < 2 {
        file.set_len(0).await?;
        info!("Downloading all...");
        download(
            config,
            &mut file,
            table,
            timestamp_range_ns.clone(),
            true,
            batch_size,
        )
        .await?;
        file.seek(SeekFrom::Start(0)).await?;
        downloaded = true;
    }
    unsafe {
        let mut start_tick = OrderFlowTick::uninitialized();
        let mut end_tick = OrderFlowTick::uninitialized();
        file.read_exact(start_tick.as_u8_slice_mut()).await?;
        file.seek(SeekFrom::End(-(OrderFlowTick::size() as i64)))
            .await?;
        file.read_exact(end_tick.as_u8_slice_mut()).await?;
        debug!(
            "saved start: {}, saved end: {}",
            start_tick.timestamp_ns.into_date_time(),
            end_tick.timestamp_ns.into_date_time(),
        );
        if downloaded {
            return Ok((file, start_tick.timestamp_ns..=end_tick.timestamp_ns));
        }
        let mut maybe_file = Some(file);
        if timestamp_range_ns.start < start_tick.timestamp_ns {
            let range = timestamp_range_ns.start..start_tick.timestamp_ns;
            let mut tmp_path = path.clone();
            tmp_path.set_extension("tmp");
            let mut tmp = File::create(&tmp_path).await?;
            let file = maybe_file.as_mut().unwrap();
            info!("Downloading first section...");
            try_join!(
                download(config, &mut tmp, table, range, true, batch_size),
                file.seek(SeekFrom::Start(0)).map_err(|e| e.into())
            )?;
            tokio::io::copy(file, &mut tmp).await?;
            try_join!(file.sync_all(), tmp.sync_all())?;
            try_join!(file.flush(), tmp.flush())?;
            drop(maybe_file.take());
            drop(tmp);
            tokio::fs::remove_file(&path).await?;
            tokio::fs::rename(&tmp_path, &path).await?;
            let file = open_rwc_async(&path).await?;
            maybe_file = Some(file);
        }
        if timestamp_range_ns.end > end_tick.timestamp_ns {
            let range = end_tick.timestamp_ns..timestamp_range_ns.end;
            let mut tmp_path = path.clone();
            tmp_path.set_extension("tmp");
            let mut tmp = open_rwc_async(&tmp_path).await?;
            let file = maybe_file.as_mut().unwrap();
            info!("Downloading last section...");
            try_join!(
                download(config, &mut tmp, table, range, false, batch_size),
                file.seek(SeekFrom::End(0)).map_err(|e| e.into())
            )?;
            tmp.seek(SeekFrom::Start(0)).await?;
            tokio::io::copy(&mut tmp, file).await?;
            // Flushing both files just to be sure
            try_join!(file.sync_all(), tmp.sync_all())?;
            try_join!(file.flush(), tmp.flush())?;
            drop(tmp);
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            tokio::fs::remove_file(&tmp_path).await?;
        }
        Ok((
            maybe_file.unwrap(),
            start_tick.timestamp_ns..=end_tick.timestamp_ns,
        ))
    }
}

#[derive(Debug, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
// Do not change, data is stored on HDD.
pub struct OrderFlowTick {
    pub timestamp_ns: i64,
    // Extra padding, and faster because of no conversion to 32 bit for register.
    pub type_mask: u32,
    pub price: f32,
    // Data is converted to float to avoid converting it when backtesting.
    pub amount: f32,
    pub n_orders: f32,
}

pub struct MssqlConnection {
    _environment: &'static odbc::Environment<odbc::Version3>,
    connection: &'static Connection<'static, AutocommitOn>,
}

impl MssqlConnection {
    pub fn new(config: &MssqlConfig) -> Result<Self> {
        let env: &'static _ = Box::leak(Box::new(create_environment_v3().map_err(|e| e.unwrap())?));
        let conn: &'static _ =
            Box::leak(Box::new(env.connect_with_connection_string(&format!(
                "driver={{ODBC Driver 17 for SQL Server}}; server={}\\{}; uid={}; pwd={}",
                config.server, config.instance, config.username, config.password
            ))?));
        Ok(Self {
            connection: conn,
            _environment: env,
        })
    }

    pub fn orderflow_loader<'a>(
        &'a self,
        table: &str,
        range: Range<DateTime<Utc>>,
        include_start: bool,
        batch_len: usize,
    ) -> impl TryStream<Item = Result<Vec<OrderFlowTick>>, Ok = Vec<OrderFlowTick>, Error = Error> + 'a
    {
        let start = range.start.format("%Y-%m-%d %H:%M:%S%.f");
        let end = range.end.format("%Y-%m-%d %H:%M:%S%.f");
        let bigger = if include_start { ">=" } else { ">" };
        let sql = format!(
            r#"SELECT Time, TypeMask, Price, Quantity, Orders FROM {table}
            WHERE Time {bigger}
            CAST('{start}' AS DATETIME2) AND
            Time <
            CAST('{end}' AS DATETIME2)
            ORDER BY Time OFFSET ? ROWS FETCH NEXT {batch_len} ROWS ONLY"#,
        );
        // TODO: this can be 4x faster if 2 threads are spawned each with own connection and then
        // we send data with channels. There is a long delay between sending a query and receiving
        // first bytes back so pipelining would improve this. This is mitigated by having a large
        // batch size of 1 GiB.
        try_stream! {
            let mut stmt = Some(Statement::with_parent(self.connection)?.prepare(&sql)?);
            let mut offset = 0u64;
            loop {
                let mut batch = Vec::with_capacity(batch_len);
                let s = stmt.take().unwrap().bind_parameter(1, &offset)?;
                match s.execute()? {
                    Data(mut s) => {
                        while let Some(mut cursor) = s.fetch()? {
                            let dt = cursor.get_data::<SqlTimestamp>(1)?.unwrap();
                            let type_mask = cursor.get_data::<u8>(2)?.unwrap() as u32;
                            let price = cursor.get_data::<f32>(3)?.unwrap() as f32;
                            let amount = cursor.get_data::<i32>(4)?.unwrap() as f32;
                            let n_orders = cursor.get_data::<i32>(5)?.unwrap() as f32;
                            let dt: NaiveDateTime =
                                NaiveDate::from_ymd_opt(dt.year as i32, dt.month as u32, dt.day as u32)
                                    .unwrap()
                                    .and_hms_nano_opt(
                                        dt.hour as u32,
                                        dt.minute as u32,
                                        dt.second as u32,
                                        dt.fraction,
                                    )
                                    .unwrap();
                            batch.push(OrderFlowTick {
                                timestamp_ns: dt.timestamp_nanos(),
                                type_mask,
                                price,
                                amount,
                                n_orders,
                            });
                        }
                        batch.last().map(|x| {
                            let dt = x.timestamp_ns.into_date_time();
                            trace!("Queried until {}", dt.format("%Y-%m-%d %H:%M:%S"));
                        });
                        let len = batch.len();
                        yield batch;
                        stmt = Some(s.close_cursor()?.reset_parameters()?);
                        offset += len as u64;
                        if len == 0 {
                            break;
                        }
                    },
                    NoData(_) => {
                        break
                    }
                }
            }
        }
    }
}
