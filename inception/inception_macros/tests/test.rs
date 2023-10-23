use inception::Query;
use inception_macros::system;

#[system]
fn empty() {}

#[system]
fn sum_simple(mut query: Query<(&i32, &mut u32)>) {
    let mut total = 0;
    query.run(|(a, b)| {
        *b += *a as u32;
        total += *b;
    });
    assert_eq!(total, 6);
}

#[system]
fn sum_entity(_a: &i32, _b: &mut u32, _other: Query<(&u8,)>) {}
