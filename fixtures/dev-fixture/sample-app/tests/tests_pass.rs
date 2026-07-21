use sample_app::add;

#[test]
fn smoke_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}
