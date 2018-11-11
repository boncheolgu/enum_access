[![Build Status](https://www.travis-ci.org/boncheolgu/enum_access.svg?branch=master)](https://www.travis-ci.org/boncheolgu/enum_access)

# EnumAccess

Custom derive for automatically generating the accessor methods for Enums.

``` rust
#[macro_use]
extern crate enum_access;

#[derive(EnumAccess, EnumDisplay)]
#[enum_access(get(name), get_some(index, value), iter(input))]
enum A<T> {
    #[enum_display("Variant1 name:{}, input:{}", input, gen)]
    Variant1 { name: String, input: i32, gen: T },
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
    #[enum_display("Variant4 index:{}, name:{}", 0, 3)]
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

    assert_eq!(v.to_string(), "Variant1 name:9, input:0");

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

    assert_eq!(v.to_string(), "");

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

    assert_eq!(v.to_string(), "Variant4 index:10, name:var4");
}
```
