pub static NNZ_TABLE: [[i16; 8]; 256] = {
    let mut table = [[0i16; 8]; 256];

    let mut i = 0;
    while i < 256 {
        let mut j = i;
        let mut k = 0;
        while j != 0 {
            table[i][k] = j.trailing_zeros() as i16;
            j &= j - 1;
            k += 1;
        }
        i += 1;
    }

    table
};