// run-rustfix
#![allow(dead_code)]
#![warn(clippy::search_is_some)]

fn main() {
    let v = vec![3, 2, 1, 0, -1, -2, -3];
    let y = &&42;

    // Check `find().is_some()`, single-line case.
    let _ = v.iter().find(|&x| *x < 0).is_some();
    let _ = (0..1).find(|x| **y == *x).is_some(); // one dereference less
    let _ = (0..1).find(|x| *x == 0).is_some();
    let _ = v.iter().find(|x| **x == 0).is_some();
    let _ = (4..5).find(|x| *x == 1 || *x == 3 || *x == 5).is_some();
    let _ = (1..3).find(|x| [1, 2, 3].contains(x)).is_some();
    let _ = (1..3).find(|x| *x == 0 || [1, 2, 3].contains(x)).is_some();
    let _ = (1..3).find(|x| [1, 2, 3].contains(x) || *x == 0).is_some();
    let _ = (1..3)
        .find(|x| [1, 2, 3].contains(x) || *x == 0 || [4, 5, 6].contains(x) || *x == -1)
        .is_some();

    // Check `position().is_some()`, single-line case.
    let _ = v.iter().position(|&x| x < 0).is_some();

    // Check `rposition().is_some()`, single-line case.
    let _ = v.iter().rposition(|&x| x < 0).is_some();

    let s1 = String::from("hello world");
    let s2 = String::from("world");
    // caller of `find()` is a `&`static str`
    let _ = "hello world".find("world").is_some();
    let _ = "hello world".find(&s2).is_some();
    let _ = "hello world".find(&s2[2..]).is_some();
    // caller of `find()` is a `String`
    let _ = s1.find("world").is_some();
    let _ = s1.find(&s2).is_some();
    let _ = s1.find(&s2[2..]).is_some();
    // caller of `find()` is slice of `String`
    let _ = s1[2..].find("world").is_some();
    let _ = s1[2..].find(&s2).is_some();
    let _ = s1[2..].find(&s2[2..]).is_some();
}

#[allow(clippy::clone_on_copy, clippy::map_clone)]
mod issue7392 {
    struct Player {
        hand: Vec<usize>,
    }
    fn filter() {
        let p = Player {
            hand: vec![1, 2, 3, 4, 5],
        };
        let filter_hand = vec![5];
        let _ = p
            .hand
            .iter()
            .filter(|c| filter_hand.iter().find(|cc| c == cc).is_some())
            .map(|c| c.clone())
            .collect::<Vec<_>>();
    }

    struct PlayerTuple {
        hand: Vec<(usize, char)>,
    }
    fn filter_tuple() {
        let p = PlayerTuple {
            hand: vec![(1, 'a'), (2, 'b'), (3, 'c'), (4, 'd'), (5, 'e')],
        };
        let filter_hand = vec![5];
        let _ = p
            .hand
            .iter()
            .filter(|(c, _)| filter_hand.iter().find(|cc| c == *cc).is_some())
            .map(|c| c.clone())
            .collect::<Vec<_>>();
    }

    fn field_projection() {
        struct Foo {
            foo: i32,
            bar: u32,
        }
        let vfoo = vec![Foo { foo: 1, bar: 2 }];
        let _ = vfoo.iter().find(|v| v.foo == 1 && v.bar == 2).is_some();

        let vfoo = vec![(42, Foo { foo: 1, bar: 2 })];
        let _ = vfoo
            .iter()
            .find(|(i, v)| *i == 42 && v.foo == 1 && v.bar == 2)
            .is_some();
    }

    fn index_projection() {
        let vfoo = vec![[0, 1, 2, 3]];
        let _ = vfoo.iter().find(|a| a[0] == 42).is_some();
    }

    #[allow(clippy::match_like_matches_macro)]
    fn slice_projection() {
        let vfoo = vec![[0, 1, 2, 3, 0, 1, 2, 3]];
        let _ = vfoo.iter().find(|sub| sub[1..4].len() == 3).is_some();
    }

    fn please(x: &u32) -> bool {
        *x == 9
    }

    fn more_projections() {
        let x = 19;
        let ppx: &u32 = &x;
        let _ = [ppx].iter().find(|ppp_x: &&&u32| please(**ppp_x)).is_some();
        let _ = [String::from("Hey hey")].iter().find(|s| s.len() == 2).is_some();
    }
}
