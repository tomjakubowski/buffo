use buffo::Buffo;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_strarray(ref strs in any::<Vec<String>>()) {
        let buffo = Buffo::str_array(strs.iter().map(|x| x.as_str()));
        let first = 0u32;
        let oob = strs.len() as u32;
        let last = if oob == 0 { 0 } else { oob - 1 };

        let get = |i| strs.get(i as usize).map(|x| x.as_str());
        prop_assert_eq!(buffo.nth_str(first), get(first));
        prop_assert_eq!(buffo.nth_str(last), get(last));
        prop_assert_eq!(buffo.nth_str(oob), get(oob));
    }
}
