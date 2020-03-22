// FIXME: Make me pass! Diff budget: 25 lines.

#[derive(Debug)]
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}

// What traits does `Duration` need to implement?
impl PartialEq for Duration {
    fn eq (&self, other: &Self) -> bool {
        match (*self, *other) {
            (Duration::MilliSeconds(m), Duration::Seconds(s)) => (m) == ((s as u64)*1000),
            (Duration::MilliSeconds(m), Duration::Minutes(t)) => (m) == ((t as u64)*1000*60),
            (Duration::Seconds(s), Duration::Minutes(t)) => (s) == ((t as u32)*60),
            (Duration::Seconds(s), Duration::MilliSeconds(m)) => ((s as u64)*1000) == (m),
            (Duration::Minutes(t), Duration::Seconds(s)) => (s) == ((t as u32)*60),
            (Duration::Minutes(t), Duration::MilliSeconds(m)) => (m) == ((t as u64)*60*1000),
            (Duration::Seconds(s), Duration::Seconds(t)) => s == t,
            (Duration::Minutes(s), Duration::Minutes(t)) => s == t,
            (Duration::MilliSeconds(s), Duration::MilliSeconds(t)) => s == t,
        }
    }
}

impl Copy for Duration {}

impl Clone for Duration {
    fn clone(&self) -> Duration {
        *self
    }
}

#[test]
fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}
