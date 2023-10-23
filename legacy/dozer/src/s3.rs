use std::sync::Arc;

use bevy::ecs::schedule::SystemDescriptor;
use db::Db;
use mouse::bytes::Bytes;
use rusoto_core::credential::StaticProvider;
use rusoto_core::Region;
use rusoto_s3::{
    CreateMultipartUploadRequest, S3Client, StreamingBody, UploadPartRequest, S3 as S3Ext,
};
use zion::prelude::*;
use zion::{handle_err, GlobalEntity, PluginLoader, Zion, ZionPlug};

use crate::common::CommonPlugin;

#[derive(Component)]
pub struct S3Credentials {
    pub username: String,
    pub password: String,
}
#[derive(Component)]
pub struct S3Region {
    pub name: String,
    pub endpoint: String,
}
#[derive(Component)]
pub struct S3(pub S3Client);

pub struct S3Plugin;

impl ZionPlug for S3Plugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(CommonPlugin)
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion.register_pipe::<S3Writer>()
            .set_stage(StartupStage::PostStartup)
            .add_startup_system(s3_startup)
            .zion()
    }
}

#[derive(Serialize, Deserialize)]
struct S3WriterArgs {
    connection: Option<Connection>,
    bucket: String,
    object: String,
}

#[derive(Serialize, Deserialize)]
struct Connection {
    region: String,
    endpoint: String,
    username: String,
    password: String,
}

pub struct S3Writer {
    reader: TopicReader<UploadPart>,
    url: String,
    bucket: String,
    key: String,
    db: Db,
    client: S3Client,
    errors: Errors<'static, 'static>,
}

#[async_trait]
impl Pipe for S3Writer {
    fn layout() -> PipeLayout
    where
        Self: Sized,
    {
        PipeLayout {
            static_estimations: None,
            kind: PipeKind::Async,
        }
    }

    fn new<'a>(builder: &mut ParamBuilder<'a>) -> AnyResult<Arc<dyn Pipe>>
    where
        Self: Sized,
    {
        let args: S3WriterArgs = builder.deserialize_args()?;
        let url;
        let client = if let Some(connection) = args.connection {
            url = connection.endpoint.clone();
            new_client(
                connection.region,
                connection.endpoint,
                connection.username,
                connection.password,
            )?
        } else {
            let world = builder.world_mut();
            let region = world.get_resource::<S3Region>().unwrap();
            url = region.endpoint.clone();
            let s3 = world.get_resource::<S3>().unwrap();
            s3.0.clone()
        };
        Ok(Arc::new(Self {
            reader: builder.get_reader()?,
            url,
            bucket: args.bucket,
            key: args.object,
            db: builder.world_mut().get_resource::<Db>().unwrap().clone(),
            client,
            errors: Errors::from(&*builder.world_mut()),
        }))
    }

    async fn spawn(self: Arc<Self>) -> AnyResult<()> {
        let this = self.clone();
        async move {
            let upload_state = match this
                .db
                .get_next_upload_state(&this.url, &this.bucket, &this.key)
                .await
            {
                Ok(id) => id,
                Err(_) => {
                    let result: AnyResult<_> = try {
                        let resp = this
                            .client
                            .create_multipart_upload(CreateMultipartUploadRequest {
                                bucket: this.bucket.clone(),
                                key: this.key.clone(),
                                ..Default::default()
                            })
                            .await?;
                        let upload_id = resp.upload_id.unwrap();
                        this.db
                            .begin_upload(&this.url, &this.bucket, &this.key, &upload_id)
                            .await?;
                        this.db
                            .get_next_upload_state(&this.url, &this.bucket, &this.key)
                            .await?
                    };
                    this.errors.handle(result).unwrap()
                }
            };
            loop {
                let guard = this.reader.read().await;
                let results = futures_util::future::join_all(guard.read_all().iter().map(|x| {
                    let data = x.data.clone();
                    let this = this.clone();
                    let upload_id = upload_state.upload_id.clone();

                    async move {
                        let result = this
                            .client
                            .upload_part(UploadPartRequest {
                                body: Some(StreamingBody::new_with_size(
                                    Box::pin(futures_util::stream::once(async move {
                                        Ok(Bytes::from_static(unsafe { data.as_ref().as_static() }))
                                    })),
                                    x.data.as_ref().len(),
                                )),
                                bucket: this.bucket.clone(),
                                content_length: Some(x.data.as_ref().len() as i64),
                                key: this.key.clone(),
                                part_number: x.part_number,
                                upload_id: upload_id,
                                ..Default::default()
                            })
                            .await;
                        result
                    }
                }))
                .await;
                for result in results {
                    this.errors.handle(result);
                }
            }
        }
        .spawn();
        Ok(())
    }

    fn system(&self) -> Option<SystemDescriptor> {
        None
    }
}

fn s3_startup(
    query: Query<(&S3Credentials, &S3Region, Entity)>,
    errors: Errors,
    mut commands: Commands,
) {
    for q in query.iter() {
        let client = ok!(new_client(
            q.1.name.clone(),
            q.1.endpoint.clone(),
            q.0.username.clone(),
            q.0.password.clone()
        ));

        commands.entity(q.2).insert(S3(client));
    }
}

fn new_client(
    region_name: String,
    endpoint: String,
    username: String,
    password: String,
) -> AnyResult<S3Client> {
    let credentials = StaticProvider::new_minimal(username, password);
    let client = rusoto_core::request::HttpClient::new()?;
    Ok(S3Client::new_with(
        client,
        credentials,
        Region::Custom {
            name: region_name,
            endpoint,
        },
    ))
}

#[async_trait]
pub trait DbS3Ext {
    async fn get_next_upload_state(
        &self,
        url: &str,
        bucket: &str,
        key: &str,
    ) -> AnyResult<UploadState>;
    async fn begin_upload(
        &self,
        url: &str,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> AnyResult<i32>;
}

#[async_trait]
impl DbS3Ext for Db {
    async fn get_next_upload_state(
        &self,
        url: &str,
        bucket: &str,
        key: &str,
    ) -> AnyResult<UploadState> {
        let mut state = query!(
            "select * from s3_get_upload_state($1, $2, $3)",
            url,
            bucket,
            key
        )
        .fetch_one(self)
        .await?;
        Ok(UploadState {
            upload_id: state.upload_id.unwrap(),
            part_number: state.part_number.unwrap() + 1,
        })
    }

    async fn begin_upload(
        &self,
        url: &str,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> AnyResult<i32> {
        let result = query!(
            "select * from s3_begin_upload($1, $2, $3, $4)",
            url,
            bucket,
            key,
            upload_id,
        )
        .fetch_one(self)
        .await?;
        Ok(result.s3_begin_upload.unwrap())
    }
}

pub struct UploadState {
    pub upload_id: String,
    pub part_number: i16,
}
