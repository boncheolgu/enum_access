#[macro_use]
extern crate enum_access;

#[derive(EnumAccess)]
#[enum_access(get(name), get_some(index, value), iter(input))]
enum A<T> {
    Variant1 {
        name: String,
        input: i32,
        gen: T,
    },
    Variant2 {
        index: u32,
        name: String,
        value: i32,
    },
    Variant3 {
        name: String,
        #[enum_alias(input, value)]
        lhs: i32,
        #[enum_alias(input)]
        rhs: i32,
        #[enum_ignore]
        input: i32,
    },
    Variant4(
        #[enum_alias(index)] u32,
        #[enum_alias(input)] i32,
        #[enum_alias(input)] i32,
        #[enum_alias(name)] String,
    ),
}

#[test]
fn it_works() {
    let mut v: A<u32> = A::Variant1 {
        name: "var1".to_string(),
        input: 9,
        gen: 0,
    };

    assert_eq!(v.get_name(), &"var1".to_string());
    assert_eq!(v.get_index(), None);
    assert_eq!(v.get_value(), None);
    assert_eq!(v.iter_inputs(), vec![&9]);

    *v.get_mut_name() = "var1'".to_string();
    assert_eq!(v.get_name(), &"var1'".to_string());

    let mut v: A<u32> = A::Variant2 {
        index: 0,
        name: "var2".to_string(),
        value: 23,
    };

    assert_eq!(v.get_name(), &"var2".to_string());
    assert_eq!(v.get_index(), Some(&0));
    assert_eq!(v.get_value(), Some(&23));
    assert_eq!(v.iter_inputs(), Vec::<&i32>::new());

    *v.get_mut_index().unwrap() = 100;
    assert_eq!(v.get_index(), Some(&100));

    let mut v: A<u32> = A::Variant3 {
        name: "var3".to_string(),
        lhs: 1,
        rhs: 2,
        input: 3,
    };

    assert_eq!(v.get_name(), &"var3".to_string());
    assert_eq!(v.get_index(), None);
    assert_eq!(v.get_value(), Some(&1));
    assert_eq!(v.iter_inputs(), vec![&1, &2]);

    for n in v.iter_mut_inputs() {
        *n += 10;
    }
    assert_eq!(v.iter_inputs(), vec![&11, &12]);

    let v: A<u32> = A::Variant4(10u32, 11i32, 12i32, "var4".to_string());
    assert_eq!(v.get_name(), &"var4".to_string());
    assert_eq!(v.get_index(), Some(&10));
    assert_eq!(v.get_value(), None);
    assert_eq!(v.iter_inputs(), vec![&11, &12]);
}
