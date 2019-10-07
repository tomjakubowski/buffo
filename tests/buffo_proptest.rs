use buffo::Buffo;
use proptest::prelude::*;

proptest! {
    #[test]
    // Simple property test that checks properties of Buffo for "all" Vec<String> inputs.
    fn test_strarray(ref strs in any::<Vec<String>>()) {
        let buffo = Buffo::str_array(strs.iter().map(|x| x.as_str()));
        let first = 0u32;
        let oob = strs.len() as u32;
        let last = if strs.len() == 0 { 0 } else { oob - 1 };
        let mid = last / 2;
        let get = |i| strs.get(i as usize).map(|x| x.as_str());

        prop_assert_eq!(buffo.nth_str(first), get(first));
        prop_assert_eq!(buffo.nth_str(mid), get(mid));
        prop_assert_eq!(buffo.nth_str(last), get(last));
        prop_assert_eq!(buffo.nth_str(oob), None);
    }
}
