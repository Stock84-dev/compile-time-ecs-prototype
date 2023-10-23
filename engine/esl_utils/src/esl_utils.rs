use core::fmt::Debug;
use std::time::Instant;

use anyhow::Result;
use esl::*;
use merovingian::hlcv::{Hlcv, MappedHlcvs};
use num_traits::ToPrimitive;
use plotters::{
    drawing::IntoDrawingArea,
    prelude::{BitMapBackend, ChartBuilder, PathElement},
    series::LineSeries,
    style::{Color, IntoFont, BLACK, RED, WHITE},
};

pub mod mssql;

pub fn change_timeframe(hlcvs: &MappedHlcvs, timeframe: u32) -> Vec<Hlcv> {
    let mut changed_hlcvs =
        vec![
            Default::default();
            merovingian::hlcv::change_timeframe_dest_len(hlcvs.len(), hlcvs.start_ts, 1, timeframe,)
        ];
    merovingian::hlcv::change_timeframe(
        hlcvs.as_ref(),
        hlcvs.start_ts,
        1,
        timeframe,
        &mut changed_hlcvs,
    );
    changed_hlcvs
}

#[system]
pub fn print_metric<M: MetricTrait>(metric: Metric<M>)
where
    M::Value: Debug,
{
    let name = core::any::type_name::<M>();
    let name = name.rfind("::").map(|x| &name[x + 2..]).unwrap_or(name);
    println!("{}: {:?}", name, metric.get());
}

#[system]
pub fn plot_tracks<T: MetricTrait>(tracks: Tracks<T>, n_trades: Metric<NTrades>)
where
    T::Value: ToPrimitive,
{
    let now = Instant::now();
    let name = core::any::type_name::<T>();
    let name = name.rfind("::").map(|x| &name[x + 2..]).unwrap_or(name);
    let tracks = tracks
        .tracks()
        .iter()
        .filter_map(|x| {
            let mut value = x.get().to_f32()?;
            // plotters hangs on these numbers
            if value == f32::INFINITY {
                value = f32::MAX;
            } else if value == f32::NEG_INFINITY {
                value = f32::MIN;
            }
            Some(value)
        })
        .take(*n_trades as usize)
        // .take_while(|x| *x != 0.)
        // skipping a few to plot more accurate results
        .skip(10)
        .collect::<Vec<_>>();
    plot(name, tracks.into_iter()).unwrap();
    println!("Plotted {} in {} ms", name, now.elapsed().as_millis());
}

#[system]
pub fn plot_tracks_optimized<T: MetricTrait>(tracks: Tracks<T>)
where
    T::Value: ToPrimitive,
{
    let now = Instant::now();
    let name = core::any::type_name::<T>();
    let name = name.rfind("::").map(|x| &name[x + 2..]).unwrap_or(name);
    let tracks = tracks.tracks();
    let mut end = tracks.len();
    for i in (0..tracks.len()).rev() {
        let value = tracks[i].get().to_f32().unwrap();
        if value != 0. {
            end = i + 1;
            break;
        }
    }
    plot(
        name,
        // skipping a few to plot more accurate results
        tracks.iter().take(end).skip(10).map(|x| {
            let mut value = x.get().to_f32().unwrap();
            // plotters hangs on these numbers
            if value == f32::INFINITY {
                value = f32::MAX;
            } else if value == f32::NEG_INFINITY {
                value = f32::MIN;
            }
            value
        }),
    )
    .unwrap();
    println!("Plotted {} in {} ms", name, now.elapsed().as_millis());
}

fn plot(name: &str, ys: impl Iterator<Item = f32> + Clone) -> Result<()> {
    let mut path = std::path::PathBuf::new();
    path.push(name);
    path.set_extension("png");
    let root = BitMapBackend::new(&path, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut max_x = 0;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    ys.clone().enumerate().for_each(|(x, y)| {
        max_x = max_x.max(x);
        if y != f32::MAX {
            max_y = max_y.max(y);
        }
        if y != f32::MIN {
            min_y = min_y.min(y);
        }
    });
    let mut chart = ChartBuilder::on(&root)
        .caption(name, ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_ranged(0.0..max_x as f32, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            ys.enumerate().map(|x| (x.0 as f32, x.1)),
            // (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
            &RED,
        ))?
        .label(name)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
