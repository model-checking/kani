/// A wrapper struct that implements `Eq` and `Ord` based on the wrapped
/// integer modulo 3. Used to test that `Iterator::max` and `Iterator::min`
/// return the correct element if some of them are equal.
#[derive(Debug)]
struct Mod3(i32);

impl PartialEq for Mod3 {
    fn eq(&self, other: &Self) -> bool {
        self.0 % 3 == other.0 % 3
    }
}

impl Eq for Mod3 {}

impl PartialOrd for Mod3 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mod3 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.0 % 3).cmp(&(other.0 % 3))
    }
}

#[test]
fn test_lt() {
    let empty: [isize; 0] = [];
    let xs = [1, 2, 3];
    let ys = [1, 2, 0];

    assert!(!xs.iter().lt(ys.iter()));
    assert!(!xs.iter().le(ys.iter()));
    assert!(xs.iter().gt(ys.iter()));
    assert!(xs.iter().ge(ys.iter()));

    assert!(ys.iter().lt(xs.iter()));
    assert!(ys.iter().le(xs.iter()));
    assert!(!ys.iter().gt(xs.iter()));
    assert!(!ys.iter().ge(xs.iter()));

    assert!(empty.iter().lt(xs.iter()));
    assert!(empty.iter().le(xs.iter()));
    assert!(!empty.iter().gt(xs.iter()));
    assert!(!empty.iter().ge(xs.iter()));

    // Sequence with NaN
    let u = [1.0f64, 2.0];
    let v = [0.0f64 / 0.0, 3.0];

    assert!(!u.iter().lt(v.iter()));
    assert!(!u.iter().le(v.iter()));
    assert!(!u.iter().gt(v.iter()));
    assert!(!u.iter().ge(v.iter()));

    let a = [0.0f64 / 0.0];
    let b = [1.0f64];
    let c = [2.0f64];

    assert!(a.iter().lt(b.iter()) == (a[0] < b[0]));
    assert!(a.iter().le(b.iter()) == (a[0] <= b[0]));
    assert!(a.iter().gt(b.iter()) == (a[0] > b[0]));
    assert!(a.iter().ge(b.iter()) == (a[0] >= b[0]));

    assert!(c.iter().lt(b.iter()) == (c[0] < b[0]));
    assert!(c.iter().le(b.iter()) == (c[0] <= b[0]));
    assert!(c.iter().gt(b.iter()) == (c[0] > b[0]));
    assert!(c.iter().ge(b.iter()) == (c[0] >= b[0]));
}

#[test]
fn test_cmp_by() {
    use core::cmp::Ordering;

    let f = |x: i32, y: i32| (x * x).cmp(&y);
    let xs = || [1, 2, 3, 4].iter().copied();
    let ys = || [1, 4, 16].iter().copied();

    assert_eq!(xs().cmp_by(ys(), f), Ordering::Less);
    assert_eq!(ys().cmp_by(xs(), f), Ordering::Greater);
    assert_eq!(xs().cmp_by(xs().map(|x| x * x), f), Ordering::Equal);
    assert_eq!(xs().rev().cmp_by(ys().rev(), f), Ordering::Greater);
    assert_eq!(xs().cmp_by(ys().rev(), f), Ordering::Less);
    assert_eq!(xs().cmp_by(ys().take(2), f), Ordering::Greater);
}

#[test]
fn test_partial_cmp_by() {
    use core::cmp::Ordering;

    let f = |x: i32, y: i32| (x * x).partial_cmp(&y);
    let xs = || [1, 2, 3, 4].iter().copied();
    let ys = || [1, 4, 16].iter().copied();

    assert_eq!(xs().partial_cmp_by(ys(), f), Some(Ordering::Less));
    assert_eq!(ys().partial_cmp_by(xs(), f), Some(Ordering::Greater));
    assert_eq!(xs().partial_cmp_by(xs().map(|x| x * x), f), Some(Ordering::Equal));
    assert_eq!(xs().rev().partial_cmp_by(ys().rev(), f), Some(Ordering::Greater));
    assert_eq!(xs().partial_cmp_by(xs().rev(), f), Some(Ordering::Less));
    assert_eq!(xs().partial_cmp_by(ys().take(2), f), Some(Ordering::Greater));

    let f = |x: f64, y: f64| (x * x).partial_cmp(&y);
    let xs = || [1.0, 2.0, 3.0, 4.0].iter().copied();
    let ys = || [1.0, 4.0, f64::NAN, 16.0].iter().copied();

    assert_eq!(xs().partial_cmp_by(ys(), f), None);
    assert_eq!(ys().partial_cmp_by(xs(), f), Some(Ordering::Greater));
}

#[test]
fn test_eq_by() {
    let f = |x: i32, y: i32| x * x == y;
    let xs = || [1, 2, 3, 4].iter().copied();
    let ys = || [1, 4, 9, 16].iter().copied();

    assert!(xs().eq_by(ys(), f));
    assert!(!ys().eq_by(xs(), f));
    assert!(!xs().eq_by(xs(), f));
    assert!(!ys().eq_by(ys(), f));

    assert!(!xs().take(3).eq_by(ys(), f));
    assert!(!xs().eq_by(ys().take(3), f));
    assert!(xs().take(3).eq_by(ys().take(3), f));
}

#[test]
fn test_iterator_nth() {
    let v: &[_] = &[0, 1, 2, 3, 4];
    for i in 0..v.len() {
        assert_eq!(v.iter().nth(i).unwrap(), &v[i]);
    }
    assert_eq!(v.iter().nth(v.len()), None);
}

#[test]
fn test_iterator_nth_back() {
    let v: &[_] = &[0, 1, 2, 3, 4];
    for i in 0..v.len() {
        assert_eq!(v.iter().nth_back(i).unwrap(), &v[v.len() - 1 - i]);
    }
    assert_eq!(v.iter().nth_back(v.len()), None);
}

#[test]
fn test_iterator_advance_by() {
    let v: &[_] = &[0, 1, 2, 3, 4];

    for i in 0..v.len() {
        let mut iter = v.iter();
        assert_eq!(iter.advance_by(i), Ok(()));
        assert_eq!(iter.next().unwrap(), &v[i]);
        assert_eq!(iter.advance_by(100), Err(v.len() - 1 - i));
    }

    assert_eq!(v.iter().advance_by(v.len()), Ok(()));
    assert_eq!(v.iter().advance_by(100), Err(v.len()));
}

#[test]
fn test_iterator_advance_back_by() {
    let v: &[_] = &[0, 1, 2, 3, 4];

    for i in 0..v.len() {
        let mut iter = v.iter();
        assert_eq!(iter.advance_back_by(i), Ok(()));
        assert_eq!(iter.next_back().unwrap(), &v[v.len() - 1 - i]);
        assert_eq!(iter.advance_back_by(100), Err(v.len() - 1 - i));
    }

    assert_eq!(v.iter().advance_back_by(v.len()), Ok(()));
    assert_eq!(v.iter().advance_back_by(100), Err(v.len()));
}

#[test]
fn test_iterator_rev_advance_back_by() {
    let v: &[_] = &[0, 1, 2, 3, 4];

    for i in 0..v.len() {
        let mut iter = v.iter().rev();
        assert_eq!(iter.advance_back_by(i), Ok(()));
        assert_eq!(iter.next_back().unwrap(), &v[i]);
        assert_eq!(iter.advance_back_by(100), Err(v.len() - 1 - i));
    }

    assert_eq!(v.iter().rev().advance_back_by(v.len()), Ok(()));
    assert_eq!(v.iter().rev().advance_back_by(100), Err(v.len()));
}

#[test]
fn test_iterator_last() {
    let v: &[_] = &[0, 1, 2, 3, 4];
    assert_eq!(v.iter().last().unwrap(), &4);
    assert_eq!(v[..1].iter().last().unwrap(), &0);
}

#[test]
fn test_iterator_max() {
    let v: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(v[..4].iter().cloned().max(), Some(3));
    assert_eq!(v.iter().cloned().max(), Some(10));
    assert_eq!(v[..0].iter().cloned().max(), None);
    assert_eq!(v.iter().cloned().map(Mod3).max().map(|x| x.0), Some(8));
}

#[test]
fn test_iterator_min() {
    let v: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(v[..4].iter().cloned().min(), Some(0));
    assert_eq!(v.iter().cloned().min(), Some(0));
    assert_eq!(v[..0].iter().cloned().min(), None);
    assert_eq!(v.iter().cloned().map(Mod3).min().map(|x| x.0), Some(0));
}

#[test]
fn test_iterator_size_hint() {
    let c = (0..).step_by(1);
    let v: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let v2 = &[10, 11, 12];
    let vi = v.iter();

    assert_eq!((0..).size_hint(), (usize::MAX, None));
    assert_eq!(c.size_hint(), (usize::MAX, None));
    assert_eq!(vi.clone().size_hint(), (10, Some(10)));

    assert_eq!(c.clone().take(5).size_hint(), (5, Some(5)));
    assert_eq!(c.clone().skip(5).size_hint().1, None);
    assert_eq!(c.clone().take_while(|_| false).size_hint(), (0, None));
    assert_eq!(c.clone().map_while(|_| None::<()>).size_hint(), (0, None));
    assert_eq!(c.clone().skip_while(|_| false).size_hint(), (0, None));
    assert_eq!(c.clone().enumerate().size_hint(), (usize::MAX, None));
    assert_eq!(c.clone().chain(vi.clone().cloned()).size_hint(), (usize::MAX, None));
    assert_eq!(c.clone().zip(vi.clone()).size_hint(), (10, Some(10)));
    assert_eq!(c.clone().scan(0, |_, _| Some(0)).size_hint(), (0, None));
    assert_eq!(c.clone().filter(|_| false).size_hint(), (0, None));
    assert_eq!(c.clone().map(|_| 0).size_hint(), (usize::MAX, None));
    assert_eq!(c.filter_map(|_| Some(0)).size_hint(), (0, None));

    assert_eq!(vi.clone().take(5).size_hint(), (5, Some(5)));
    assert_eq!(vi.clone().take(12).size_hint(), (10, Some(10)));
    assert_eq!(vi.clone().skip(3).size_hint(), (7, Some(7)));
    assert_eq!(vi.clone().skip(12).size_hint(), (0, Some(0)));
    assert_eq!(vi.clone().take_while(|_| false).size_hint(), (0, Some(10)));
    assert_eq!(vi.clone().map_while(|_| None::<()>).size_hint(), (0, Some(10)));
    assert_eq!(vi.clone().skip_while(|_| false).size_hint(), (0, Some(10)));
    assert_eq!(vi.clone().enumerate().size_hint(), (10, Some(10)));
    assert_eq!(vi.clone().chain(v2).size_hint(), (13, Some(13)));
    assert_eq!(vi.clone().zip(v2).size_hint(), (3, Some(3)));
    assert_eq!(vi.clone().scan(0, |_, _| Some(0)).size_hint(), (0, Some(10)));
    assert_eq!(vi.clone().filter(|_| false).size_hint(), (0, Some(10)));
    assert_eq!(vi.clone().map(|&i| i + 1).size_hint(), (10, Some(10)));
    assert_eq!(vi.filter_map(|_| Some(0)).size_hint(), (0, Some(10)));
}

#[test]
fn test_all() {
    let v: Box<[isize]> = Box::new([1, 2, 3, 4, 5]);
    assert!(v.iter().all(|&x| x < 10));
    assert!(!v.iter().all(|&x| x % 2 == 0));
    assert!(!v.iter().all(|&x| x > 100));
    assert!(v[..0].iter().all(|_| panic!()));
}

#[test]
fn test_any() {
    let v: Box<[isize]> = Box::new([1, 2, 3, 4, 5]);
    assert!(v.iter().any(|&x| x < 10));
    assert!(v.iter().any(|&x| x % 2 == 0));
    assert!(!v.iter().any(|&x| x > 100));
    assert!(!v[..0].iter().any(|_| panic!()));
}

#[test]
fn test_find() {
    let v: &[isize] = &[1, 3, 9, 27, 103, 14, 11];
    assert_eq!(*v.iter().find(|&&x| x & 1 == 0).unwrap(), 14);
    assert_eq!(*v.iter().find(|&&x| x % 3 == 0).unwrap(), 3);
    assert!(v.iter().find(|&&x| x % 12 == 0).is_none());
}

#[test]
fn test_try_find() {
    let xs: &[isize] = &[];
    assert_eq!(xs.iter().try_find(testfn), Ok(None));
    let xs: &[isize] = &[1, 2, 3, 4];
    assert_eq!(xs.iter().try_find(testfn), Ok(Some(&2)));
    let xs: &[isize] = &[1, 3, 4];
    assert_eq!(xs.iter().try_find(testfn), Err(()));

    let xs: &[isize] = &[1, 2, 3, 4, 5, 6, 7];
    let mut iter = xs.iter();
    assert_eq!(iter.try_find(testfn), Ok(Some(&2)));
    assert_eq!(iter.try_find(testfn), Err(()));
    assert_eq!(iter.next(), Some(&5));

    fn testfn(x: &&isize) -> Result<bool, ()> {
        if **x == 2 {
            return Ok(true);
        }
        if **x == 4 {
            return Err(());
        }
        Ok(false)
    }
}

#[test]
fn test_try_find_api_usability() -> Result<(), Box<dyn std::error::Error>> {
    let a = ["1", "2"];

    let is_my_num = |s: &str, search: i32| -> Result<bool, std::num::ParseIntError> {
        Ok(s.parse::<i32>()? == search)
    };

    let val = a.iter().try_find(|&&s| is_my_num(s, 2))?;
    assert_eq!(val, Some(&"2"));

    Ok(())
}

#[test]
fn test_position() {
    let v = &[1, 3, 9, 27, 103, 14, 11];
    assert_eq!(v.iter().position(|x| *x & 1 == 0).unwrap(), 5);
    assert_eq!(v.iter().position(|x| *x % 3 == 0).unwrap(), 1);
    assert!(v.iter().position(|x| *x % 12 == 0).is_none());
}

#[test]
fn test_count() {
    let xs = &[1, 2, 2, 1, 5, 9, 0, 2];
    assert_eq!(xs.iter().filter(|x| **x == 2).count(), 3);
    assert_eq!(xs.iter().filter(|x| **x == 5).count(), 1);
    assert_eq!(xs.iter().filter(|x| **x == 95).count(), 0);
}

#[test]
fn test_max_by_key() {
    let xs: &[isize] = &[-3, 0, 1, 5, -10];
    assert_eq!(*xs.iter().max_by_key(|x| x.abs()).unwrap(), -10);
}

#[test]
fn test_max_by() {
    let xs: &[isize] = &[-3, 0, 1, 5, -10];
    assert_eq!(*xs.iter().max_by(|x, y| x.abs().cmp(&y.abs())).unwrap(), -10);
}

#[test]
fn test_min_by_key() {
    let xs: &[isize] = &[-3, 0, 1, 5, -10];
    assert_eq!(*xs.iter().min_by_key(|x| x.abs()).unwrap(), 0);
}

#[test]
fn test_min_by() {
    let xs: &[isize] = &[-3, 0, 1, 5, -10];
    assert_eq!(*xs.iter().min_by(|x, y| x.abs().cmp(&y.abs())).unwrap(), 0);
}

#[test]
fn test_by_ref() {
    let mut xs = 0..10;
    // sum the first five values
    let partial_sum = xs.by_ref().take(5).fold(0, |a, b| a + b);
    assert_eq!(partial_sum, 10);
    assert_eq!(xs.next(), Some(5));
}

#[test]
fn test_is_sorted() {
    assert!([1, 2, 2, 9].iter().is_sorted());
    assert!(![1, 3, 2].iter().is_sorted());
    assert!([0].iter().is_sorted());
    assert!(std::iter::empty::<i32>().is_sorted());
    assert!(![0.0, 1.0, f32::NAN].iter().is_sorted());
    assert!([-2, -1, 0, 3].iter().is_sorted());
    assert!(![-2i32, -1, 0, 3].iter().is_sorted_by_key(|n| n.abs()));
    assert!(!["c", "bb", "aaa"].iter().is_sorted());
    assert!(["c", "bb", "aaa"].iter().is_sorted_by_key(|s| s.len()));
}

#[test]
fn test_partition() {
    fn check(xs: &mut [i32], ref p: impl Fn(&i32) -> bool, expected: usize) {
        let i = xs.iter_mut().partition_in_place(p);
        assert_eq!(expected, i);
        assert!(xs[..i].iter().all(p));
        assert!(!xs[i..].iter().any(p));
        assert!(xs.iter().is_partitioned(p));
        if i == 0 || i == xs.len() {
            assert!(xs.iter().rev().is_partitioned(p));
        } else {
            assert!(!xs.iter().rev().is_partitioned(p));
        }
    }

    check(&mut [], |_| true, 0);
    check(&mut [], |_| false, 0);

    check(&mut [0], |_| true, 1);
    check(&mut [0], |_| false, 0);

    check(&mut [-1, 1], |&x| x > 0, 1);
    check(&mut [-1, 1], |&x| x < 0, 1);

    let ref mut xs = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    check(xs, |_| true, 10);
    check(xs, |_| false, 0);
    check(xs, |&x| x % 2 == 0, 5); // evens
    check(xs, |&x| x % 2 == 1, 5); // odds
    check(xs, |&x| x % 3 == 0, 4); // multiple of 3
    check(xs, |&x| x % 4 == 0, 3); // multiple of 4
    check(xs, |&x| x % 5 == 0, 2); // multiple of 5
    check(xs, |&x| x < 3, 3); // small
    check(xs, |&x| x > 6, 3); // large
}

#[test]
fn test_iterator_rev_advance_by() {
    let v: &[_] = &[0, 1, 2, 3, 4];

    for i in 0..v.len() {
        let mut iter = v.iter().rev();
        assert_eq!(iter.advance_by(i), Ok(()));
        assert_eq!(iter.next().unwrap(), &v[v.len() - 1 - i]);
        assert_eq!(iter.advance_by(100), Err(v.len() - 1 - i));
    }

    assert_eq!(v.iter().rev().advance_by(v.len()), Ok(()));
    assert_eq!(v.iter().rev().advance_by(100), Err(v.len()));
}

#[test]
fn test_find_map() {
    let xs: &[isize] = &[];
    assert_eq!(xs.iter().find_map(half_if_even), None);
    let xs: &[isize] = &[3, 5];
    assert_eq!(xs.iter().find_map(half_if_even), None);
    let xs: &[isize] = &[4, 5];
    assert_eq!(xs.iter().find_map(half_if_even), Some(2));
    let xs: &[isize] = &[3, 6];
    assert_eq!(xs.iter().find_map(half_if_even), Some(3));

    let xs: &[isize] = &[1, 2, 3, 4, 5, 6, 7];
    let mut iter = xs.iter();
    assert_eq!(iter.find_map(half_if_even), Some(1));
    assert_eq!(iter.find_map(half_if_even), Some(2));
    assert_eq!(iter.find_map(half_if_even), Some(3));
    assert_eq!(iter.next(), Some(&7));

    fn half_if_even(x: &isize) -> Option<isize> {
        if x % 2 == 0 { Some(x / 2) } else { None }
    }
}

#[test]
fn test_try_reduce() {
    let v: Vec<usize> = vec![1, 2, 3, 4, 5];
    let sum = v.into_iter().try_reduce(|x, y| x.checked_add(y));
    assert_eq!(sum, Some(Some(15)));

    let v: Vec<usize> = vec![1, 2, 3, 4, 5, usize::MAX];
    let sum = v.into_iter().try_reduce(|x, y| x.checked_add(y));
    assert_eq!(sum, None);

    let v: Vec<usize> = Vec::new();
    let sum = v.into_iter().try_reduce(|x, y| x.checked_add(y));
    assert_eq!(sum, Some(None));

    let v = vec!["1", "2", "3", "4", "5"];
    let max = v.into_iter().try_reduce(|x, y| {
        if x.parse::<usize>().ok()? > y.parse::<usize>().ok()? { Some(x) } else { Some(y) }
    });
    assert_eq!(max, Some(Some("5")));

    let v = vec!["1", "2", "3", "4", "5"];
    let max: Result<Option<_>, <usize as std::str::FromStr>::Err> =
        v.into_iter().try_reduce(|x, y| {
            if x.parse::<usize>()? > y.parse::<usize>()? { Ok(x) } else { Ok(y) }
        });
    assert_eq!(max, Ok(Some("5")));
}

#[test]
fn test_iterator_len() {
    let v: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(v[..4].iter().count(), 4);
    assert_eq!(v[..10].iter().count(), 10);
    assert_eq!(v[..0].iter().count(), 0);
}

#[test]
fn test_collect() {
    let a = vec![1, 2, 3, 4, 5];
    let b: Vec<isize> = a.iter().cloned().collect();
    assert!(a == b);
}
