use crate::score::Score;

pub fn wdl_params(mat: i16) -> (f64, f64) {
    let a = [-176.86105327, 445.05551947, -552.76358726, 529.72457827];
    let b = [9.54877939, -65.95697697, 120.90767109, 34.73323854];

    let m = mat.clamp(17, 78) as f64 / 58.0;

    return (
        ((a[0] * m + a[1]) * m + a[2]) * m + a[3],
        ((b[0] * m + b[1]) * m + b[2]) * m + b[3],
    );
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
    return Score::clamp_nomate(f64::round(normalized * 100.0) as i16);
}
