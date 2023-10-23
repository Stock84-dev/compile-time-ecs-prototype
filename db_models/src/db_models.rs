#[derive(Serialize, Deserialize)]
pub struct Tick {
    time: DateTime<Utc>,
    type_mask: u8,
    price: f32,
    quantity: f32,
    order: f32,
}
