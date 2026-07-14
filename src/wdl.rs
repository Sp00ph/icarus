use crate::score::Score;

pub fn wdl_params(mat: i16) -> (f64, f64) {
    let a = [-36.24535760, 49.95573192, -171.69014901, 476.10858668];
    let b = [-82.69201609, 218.83093958, -140.40735385, 108.56006144];

    let m = mat.clamp(17, 78) as f64 / 58.0;

    (
        ((a[0] * m + a[1]) * m + a[2]) * m + a[3],
        ((b[0] * m + b[1]) * m + b[2]) * m + b[3],
    )
}

pub fn wdl_model(score: Score, mat: i16) -> (i16, i16) {
    let (a, b) = wdl_params(mat);
    let x = score.0 as f64;

    (
        f64::round(1000.0 / (1.0 + f64::exp((a - x) / b))) as i16,
        f64::round(1000.0 / (1.0 + f64::exp((a + x) / b))) as i16,
    )
}

pub fn normalize(score: Score, mat: i16) -> Score {
    if score == Score::ZERO || score.is_mate() {
        return score;
    }

    let a = wdl_params(mat).0;
    let normalized = (score.0 as f64) / a;
    Score::clamp_nomate(f64::round(normalized * 100.0) as i16)
}
