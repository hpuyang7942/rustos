// FIXME: Make me pass! Diff budget: 30 lines.

#[derive(Default)]
struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

impl Builder {
    // fn string(...
    fn string<T : AsRef<str>>(mut self, input: T) -> Builder {
        self.string = Some(input.as_ref().to_string());
        self
    }
    // fn number(...
    fn number(mut self, input: i32) -> Builder {
        self.number = Some(input as usize);
        self
    }
}

impl ToString for Builder {
    // Implement the trait
    fn to_string(&self) -> String {
        let _string: String = match &self.string {
            Some(my_string) => my_string.to_string(),
            None => "".to_owned()
        };
        let _number = match self.number {
            Some(my_number) => my_number.to_string(),
            None => "".to_string()
        };
        if _string != "" && _number != "" {
            _string + " " + &_number
        }
        else if _string == "" {
            _number
        }
        else {
            _string
        }
    }
}

// Do not modify this function.
#[test]
fn builder() {
    let empty = Builder::default().to_string();
    assert_eq!(empty, "");

    let just_str = Builder::default().string("hi").to_string();
    assert_eq!(just_str, "hi");

    let just_num = Builder::default().number(254).to_string();
    assert_eq!(just_num, "254");

    let a = Builder::default()
        .string("hello, world!")
        .number(200)
        .to_string();

    assert_eq!(a, "hello, world! 200");

    let b = Builder::default()
        .string("hello, world!")
        .number(200)
        .string("bye now!")
        .to_string();

    assert_eq!(b, "bye now! 200");

    let c = Builder::default()
        .string("heap!".to_owned())
        .to_string();

    assert_eq!(c, "heap!");
}
