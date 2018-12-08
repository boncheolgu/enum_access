#[macro_use]
extern crate enum_access;

#[derive(EnumAccess, EnumDisplay)]
#[enum_access(get(name), get_some(index, value), iter(inputs))]
enum A<T> {
    #[enum_display("Variant1 name:{}, input:{}", input, gen)]
    Variant1 {
        name: String,
        #[enum_alias(inputs)]
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
        #[enum_alias(inputs, value)]
        lhs: i32,
        #[enum_alias(inputs)]
        rhs: i32,
        #[enum_ignore]
        input: i32,
    },
    #[enum_display("Variant4 index:{}, name:{}", 0, 3)]
    Variant4(
        #[enum_alias(index)] u32,
        #[enum_alias(inputs)] i32,
        #[enum_alias(inputs)] i32,
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

    assert_eq!(v.name(), &"var1".to_string());
    assert_eq!(v.index(), None);
    assert_eq!(v.value(), None);
    assert_eq!(v.inputs(), vec![&9]);

    assert_eq!(v.to_string(), "Variant1 name:9, input:0");

    *v.name_mut() = "var1'".to_string();
    assert_eq!(v.name(), &"var1'".to_string());

    let mut v: A<u32> = A::Variant2 {
        index: 0,
        name: "var2".to_string(),
        value: 23,
    };

    assert_eq!(v.name(), &"var2".to_string());
    assert_eq!(v.index(), Some(&0));
    assert_eq!(v.value(), Some(&23));
    assert_eq!(v.inputs(), Vec::<&i32>::new());

    assert_eq!(v.to_string(), "");

    *v.index_mut().unwrap() = 100;
    assert_eq!(v.index(), Some(&100));

    let mut v: A<u32> = A::Variant3 {
        name: "var3".to_string(),
        lhs: 1,
        rhs: 2,
        input: 3,
    };

    assert_eq!(v.name(), &"var3".to_string());
    assert_eq!(v.index(), None);
    assert_eq!(v.value(), Some(&1));
    assert_eq!(v.inputs(), vec![&1, &2]);

    for n in v.inputs_mut() {
        *n += 10;
    }
    assert_eq!(v.inputs(), vec![&11, &12]);

    let v: A<u32> = A::Variant4(10u32, 11i32, 12i32, "var4".to_string());
    assert_eq!(v.name(), &"var4".to_string());
    assert_eq!(v.index(), Some(&10));
    assert_eq!(v.value(), None);
    assert_eq!(v.inputs(), vec![&11, &12]);

    assert_eq!(v.to_string(), "Variant4 index:10, name:var4");
}
