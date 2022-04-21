macro_rules! make_u16 {
    ($h: expr, $l: expr) => {
        (($h as u16) << 8) | $l as u16
    }
}

macro_rules! is_set {
    ($n: expr, $flag: expr) => {
        ($n & ($flag as u8)) == $flag
    }
}

macro_rules! is_not_set {
    ($n: expr, $flag: expr) => {
        ($n & ($flag as u8)) != $flag
    }
}
