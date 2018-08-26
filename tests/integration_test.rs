#[macro_use]
extern crate enum_access;

#[derive(EnumAccess)]
#[enum_get(name)]
#[enum_get_some(index)]
#[enum_iter(input)]
enum A {
    Variant1 {
        name: String,
        input: i32,
    },
    Variant2 {
        index: u32,
        name: String,
    },
    Variant3 {
        name: String,
        #[enum_alias(input)]
        lhs: i32,
        #[enum_alias(input)]
        rhs: i32,
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
    let mut v = A::Variant1 {
        name: "var1".to_string(),
        input: 9,
    };

    assert_eq!(v.get_name(), &"var1".to_string());
    assert_eq!(v.get_index(), None);
    assert_eq!(v.iter_inputs(), vec![&9]);

    *v.get_mut_name() = "var1'".to_string();
    assert_eq!(v.get_name(), &"var1'".to_string());

    let mut v = A::Variant2 {
        index: 0,
        name: "var2".to_string(),
    };

    assert_eq!(v.get_name(), &"var2".to_string());
    assert_eq!(v.get_index(), Some(&0));
    assert_eq!(v.iter_inputs(), Vec::<&i32>::new());

    *v.get_mut_index().unwrap() = 100;
    assert_eq!(v.get_index(), Some(&100));

    let mut v = A::Variant3 {
        name: "var3".to_string(),
        lhs: 1,
        rhs: 2,
    };

    assert_eq!(v.get_name(), &"var3".to_string());
    assert_eq!(v.get_index(), None);
    assert_eq!(v.iter_inputs(), vec![&1, &2]);

    for n in v.iter_mut_inputs() {
        *n += 10;
    }
    assert_eq!(v.iter_inputs(), vec![&11, &12]);

    let v = A::Variant4(10u32, 11i32, 12i32, "var4".to_string());
    assert_eq!(v.get_name(), &"var4".to_string());
    assert_eq!(v.get_index(), Some(&10));
    assert_eq!(v.iter_inputs(), vec![&11, &12]);
}
