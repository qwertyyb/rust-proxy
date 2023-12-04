fn get_bit(data: u16, pos: usize) -> bool {
    ((data & (1 << pos)) >> pos) == 1
}
fn get_value(data: u16, pos: usize, len: usize) -> u16 {
    data << (16 - pos - len) >> (15 - pos)
}
fn set_bit(data: u16, pos: usize, value: bool) -> u16 {
    if value {
        data | (1 << pos)
    } else {
        data & !(1 << pos)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_bit() {
        let cases: [(u16, usize, bool); 2] = [(0xf0f0, 4, true), (0xf0f0, 9, false)];
        for (data, pos, result) in cases {
            assert_eq!(
                get_bit(data, pos),
                result,
                "get_bit({data:02X}, {pos}) != {result}"
            )
        }
    }

    #[test]
    fn test_get_value() {
        // 0xf2: 11110010
        let cases: [(u16, usize, usize, u16); 4] = [
            (0xf2, 0, 1, 0),
            (0xf2, 1, 2, 1),
            (0xf2, 1, 3, 1),
            (0xf2, 1, 4, 0x1001),
        ];
        for (data, pos, len, result) in cases {
            assert_eq!(
                get_value(data, pos, len),
                result,
                "get_value({data:02X}, {pos}, {len}) != {result}"
            );
        }
    }

    #[test]
    fn test_set_bit() {
        let cases: [(u16, usize, bool, u16); 4] = [
            (0x11, 0, false, 16),
            (0x11, 4, false, 1),
            (0x01, 0, false, 0),
            (0x01, 1, true, 3),
        ];
        for (data, pos, value, result) in cases {
            assert_eq!(
                set_bit(data, pos, value),
                result,
                "set_bit({data:02X}, {pos}, {value}) != {result}"
            );
        }
    }
}
