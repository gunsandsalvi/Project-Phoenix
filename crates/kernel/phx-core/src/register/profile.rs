use libm::{exp, pow, sqrt};
use phx_macros::clause;
use phx_rand::{Draws, normal_quantile, open_unit};
use toml::Value;

use crate::consts::{DECIMAL_RADIX, DECIMAL_RADIX_F64, PERCENT_F64};

/// The scale a profile's value is drawn on, so that a draw anywhere on it maps into the value's own domain and
/// nothing is clamped: a positive value in logs, a percentage or a share by its logit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    Identity,
    Log,
    LogitPercent,
    LogitShare,
}

/// One value of a joint profile, on its transformed scale: its mean and dispersion across the group's countries,
/// and how many countries they were measured over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileValue {
    pub name: String,
    pub transform: Transform,
    pub mean: i64,
    pub sd: i64,
    pub countries: u32,
}

/// A country group's joint profile: its values' means and dispersions and their correlations, each number to `exp`
/// decimal places, the correlations row by row.
#[clause("GEN.12")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointProfile {
    pub values: Vec<ProfileValue>,
    pub correlation: Vec<i64>,
    pub exp: u8,
}

impl JointProfile {
    /// The correlation of two values, raw at the profile's scale.
    #[must_use]
    pub fn correlation(&self, a: usize, b: usize) -> Option<i64> {
        self.correlation.get(a * self.values.len() + b).copied()
    }

    #[must_use]
    pub fn index(&self, name: &str) -> Option<usize> {
        self.values.iter().position(|v| v.name == name)
    }
}

fn transform(text: &str) -> Result<Transform, String> {
    match text {
        "identity" => Ok(Transform::Identity),
        "log" => Ok(Transform::Log),
        "logit_percent" => Ok(Transform::LogitPercent),
        "logit_share" => Ok(Transform::LogitShare),
        other => Err(format!("transform `{other}` is not one of identity, log, logit_percent, logit_share")),
    }
}

/// A profile from data: every value named once with a positive dispersion, and a correlation matrix over them that
/// is symmetric with a unit diagonal and no entry beyond one.
pub(crate) fn parse(
    v: &Value,
    exp: u8,
    decimal: impl Fn(&Value, u8) -> Result<i64, String>,
) -> Result<JointProfile, String> {
    let t = v.as_table().ok_or("a profile is a table")?;
    let items = t.get("values").and_then(Value::as_array).ok_or("a profile has `values`")?;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let get = |k: &str| item.get(k).ok_or_else(|| format!("a profile value without `{k}`"));
        let name = get("name")?.as_str().ok_or("a value's name is text")?.to_owned();
        let countries = get("countries")?
            .as_integer()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("a value's `countries` is a count")?;
        let sd = decimal(get("sd")?, exp)?;
        if sd <= 0 {
            return Err(format!("`{name}` has no dispersion"));
        }
        let transform = transform(get("transform")?.as_str().ok_or("a transform is text")?)?;
        values.push(ProfileValue { name, transform, mean: decimal(get("mean")?, exp)?, sd, countries });
    }
    for (i, a) in values.iter().enumerate() {
        if values.iter().skip(i + 1).any(|b| b.name == a.name) {
            return Err(format!("`{}` named twice in a profile", a.name));
        }
    }
    let rows = t.get("correlation").and_then(Value::as_array).ok_or("a profile has `correlation`")?;
    if rows.len() != values.len() {
        return Err(format!("{} correlation rows for {} values", rows.len(), values.len()));
    }
    let one = DECIMAL_RADIX.checked_pow(u32::from(exp)).ok_or("a profile's decimal places overflow")?;
    let mut correlation = Vec::with_capacity(values.len() * values.len());
    for row in rows {
        let row = row.as_array().ok_or("a correlation row is a list")?;
        if row.len() != values.len() {
            return Err(format!("a correlation row of {} for {} values", row.len(), values.len()));
        }
        for x in row {
            let r = decimal(x, exp)?;
            if r.abs() > one {
                return Err("a correlation beyond one".to_owned());
            }
            correlation.push(r);
        }
    }
    let n = values.len();
    for i in 0..n {
        if correlation.get(i * n + i) != Some(&one) {
            return Err(format!("`{}`'s correlation with itself is not one", values.get(i).map_or("", |v| &v.name)));
        }
        for j in 0..i {
            if correlation.get(i * n + j) != correlation.get(j * n + i) {
                return Err("a correlation matrix that is not symmetric".to_owned());
            }
        }
    }
    Ok(JointProfile { values, correlation, exp })
}

/// A value the setup fixes the level of: its place in the profile, and which part of its group's distribution the
/// level asks for, from the bottom, the distribution cut into as many equal parts as a choice has levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pinned {
    pub index: usize,
    pub part: u8,
}

/// A profile's numbers as reals on the transformed scale: means, dispersions and the covariance matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct Moments {
    pub mean: Vec<f64>,
    pub cov: Vec<f64>,
}

fn real(raw: i64, exp: u8) -> f64 {
    phx_rand::float::from_i64(raw) / pow(DECIMAL_RADIX_F64, f64::from(exp))
}

/// A square matrix of `size` rows, stored row by row, read with zero outside it.
#[derive(Clone, Copy, Debug)]
struct Square<'a> {
    cells: &'a [f64],
    size: usize,
}

impl Square<'_> {
    fn at(self, row: usize, col: usize) -> f64 {
        self.cells.get(row * self.size + col).copied().unwrap_or(0.0)
    }
}

fn at(values: &[f64], index: usize) -> f64 {
    values.get(index).copied().unwrap_or(0.0)
}

/// The profile's means and covariances, from its dispersions and correlations.
#[must_use]
pub fn moments(profile: &JointProfile) -> Moments {
    let size = profile.values.len();
    let sd: Vec<f64> = profile.values.iter().map(|v| real(v.sd, profile.exp)).collect();
    let mut cov = Vec::with_capacity(size * size);
    for row in 0..size {
        for col in 0..size {
            let corr = profile.correlation(row, col).map_or(0.0, |r| real(r, profile.exp));
            cov.push(corr * at(&sd, row) * at(&sd, col));
        }
    }
    Moments { mean: profile.values.iter().map(|v| real(v.mean, profile.exp)).collect(), cov }
}

/// The lower Cholesky factor of a symmetric matrix of `size` rows, row by row; `None` when the matrix is not
/// positive definite.
#[must_use]
pub fn cholesky(matrix: &[f64], size: usize) -> Option<Vec<f64>> {
    let mut lower = vec![0.0; size * size];
    for row in 0..size {
        for col in 0..=row {
            let factor = Square { cells: &lower, size };
            let dot: f64 = (0..col).map(|k| factor.at(row, k) * factor.at(col, k)).sum();
            let entry = *matrix.get(row * size + col)?;
            let value = if row == col {
                let pivot = entry - dot;
                if pivot <= 0.0 {
                    return None;
                }
                sqrt(pivot)
            } else {
                (entry - dot) / factor.at(col, col)
            };
            *lower.get_mut(row * size + col)? = value;
        }
    }
    Some(lower)
}

/// Solves `L Lᵀ x = rhs` for `x`, given the Cholesky factor `L`.
fn solve(factor: Square<'_>, rhs: &[f64]) -> Vec<f64> {
    let size = factor.size;
    let mut forward = vec![0.0; size];
    for row in 0..size {
        let done: f64 = (0..row).map(|k| factor.at(row, k) * at(&forward, k)).sum();
        if let Some(slot) = forward.get_mut(row) {
            *slot = (at(rhs, row) - done) / factor.at(row, row);
        }
    }
    let mut back = vec![0.0; size];
    for row in (0..size).rev() {
        let done: f64 = (row + 1..size).map(|k| factor.at(k, row) * at(&back, k)).sum();
        if let Some(slot) = back.get_mut(row) {
            *slot = (at(&forward, row) - done) / factor.at(row, row);
        }
    }
    back
}

/// The distribution of the other values given some values: the conditional mean and covariance of a joint normal,
/// `μ_B + Σ_BA Σ_AA⁻¹ (x_A − μ_A)` and `Σ_BB − Σ_BA Σ_AA⁻¹ Σ_AB`, over `rest`, the indices not given.
///
/// # Errors
/// When the given values' covariance is not positive definite.
pub fn conditional(moments: &Moments, given: &[(usize, f64)]) -> Result<(Vec<usize>, Moments), String> {
    let full = Square { cells: &moments.cov, size: moments.mean.len() };
    let known: Vec<usize> = given.iter().map(|(index, _)| *index).collect();
    let rest: Vec<usize> = (0..full.size).filter(|index| !known.contains(index)).collect();
    let known_cov: Vec<f64> = known.iter().flat_map(|a| known.iter().map(move |b| full.at(*a, *b))).collect();
    let factor = cholesky(&known_cov, known.len()).ok_or("the given values' covariance is not positive definite")?;
    let factor = Square { cells: &factor, size: known.len() };
    let deviation: Vec<f64> = given.iter().map(|(index, value)| value - at(&moments.mean, *index)).collect();
    let weights = solve(factor, &deviation);
    let mean = rest
        .iter()
        .map(|b| at(&moments.mean, *b) + known.iter().zip(&weights).map(|(a, w)| full.at(*b, *a) * w).sum::<f64>())
        .collect();
    let through: Vec<Vec<f64>> =
        rest.iter().map(|b| solve(factor, &known.iter().map(|a| full.at(*a, *b)).collect::<Vec<_>>())).collect();
    let mut cov = Vec::with_capacity(rest.len() * rest.len());
    for (row, b) in rest.iter().enumerate() {
        let solved = through.get(row).map_or(&[][..], Vec::as_slice);
        for c in &rest {
            let explained: f64 = known.iter().zip(solved).map(|(a, x)| full.at(*c, *a) * x).sum();
            cov.push(full.at(*b, *c) - explained);
        }
    }
    Ok((rest, Moments { mean, cov }))
}

/// A joint normal draw from standard normals: `μ + L z`.
#[must_use]
pub fn from_normals(moments: &Moments, factor: &[f64], normals: &[f64]) -> Vec<f64> {
    let lower = Square { cells: factor, size: moments.mean.len() };
    (0..lower.size)
        .map(|row| at(&moments.mean, row) + (0..=row).map(|k| lower.at(row, k) * at(normals, k)).sum::<f64>())
        .collect()
}

/// A value on its own scale from its transformed one.
#[must_use]
pub fn natural(transform: Transform, value: f64) -> f64 {
    match transform {
        Transform::Identity => value,
        Transform::Log => exp(value),
        Transform::LogitPercent => PERCENT_F64 / (1.0 + exp(-value)),
        Transform::LogitShare => 1.0 / (1.0 + exp(-value)),
    }
}

/// A country's derived values on their own scales: each pinned value drawn within its part of its own marginal
/// distribution, cut into `levels` parts of equal probability; then the rest drawn jointly given them, so values
/// that go together stay together and nothing is clamped.
///
/// # Errors
/// When the pinned values' covariance, or the rest's given them, is not positive definite.
#[clause("GEN.15")]
pub fn draw_profile(
    profile: &JointProfile,
    pinned: &[Pinned],
    levels: u8,
    draws: &mut Draws,
) -> Result<Vec<f64>, String> {
    let all = moments(profile);
    let full = Square { cells: &all.cov, size: all.mean.len() };
    let mut given = Vec::with_capacity(pinned.len());
    for pin in pinned {
        let unit = (f64::from(pin.part) + open_unit(draws)) / f64::from(levels);
        let sd = sqrt(full.at(pin.index, pin.index));
        given.push((pin.index, at(&all.mean, pin.index) + sd * normal_quantile(unit)));
    }
    let (rest, given_rest) = conditional(&all, &given)?;
    let factor = cholesky(&given_rest.cov, rest.len()).ok_or("the profile's covariance is not positive definite")?;
    let normals: Vec<f64> = rest.iter().map(|_| normal_quantile(open_unit(draws))).collect();
    let sampled = from_normals(&given_rest, &factor, &normals);
    let mut transformed = vec![0.0; full.size];
    for (index, value) in given.iter().copied().chain(rest.iter().copied().zip(sampled)) {
        if let Some(slot) = transformed.get_mut(index) {
            *slot = value;
        }
    }
    Ok(profile.values.iter().zip(transformed).map(|(v, t)| natural(v.transform, t)).collect())
}

#[cfg(test)]
mod tests {
    use super::{Moments, Transform, cholesky, conditional, from_normals, natural};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn conditional_draw_matches_closed_form() {
        let (m1, m2, s1, s2, rho) = (1.0, -2.0, 0.5, 3.0, 0.6);
        let m = Moments { mean: vec![m1, m2], cov: vec![s1 * s1, rho * s1 * s2, rho * s1 * s2, s2 * s2] };
        let (rest, c) = conditional(&m, &[(0, 1.8)]).unwrap();
        assert_eq!(rest, vec![1]);
        assert!(close(c.mean[0], m2 + rho * s2 / s1 * (1.8 - m1)));
        assert!(close(c.cov[0], s2 * s2 * (1.0 - rho * rho)));
        let three = Moments { mean: vec![0.0, 0.0, 0.0], cov: vec![1.0, 0.5, 0.2, 0.5, 1.0, 0.3, 0.2, 0.3, 1.0] };
        let (rest, c) = conditional(&three, &[(0, 1.0), (1, -1.0)]).unwrap();
        let (a, b) = ((0.2 - 0.3 * 0.5) / 0.75, (0.3 - 0.2 * 0.5) / 0.75);
        assert_eq!(rest, vec![2]);
        assert!(close(c.mean[0], a - b));
        assert!(close(c.cov[0], 1.0 - (a * 0.2 + b * 0.3)));
    }

    #[test]
    fn profile_draw_keeps_correlations() {
        let m = Moments { mean: vec![1.0, 2.0], cov: vec![4.0, 1.2, 1.2, 1.0] };
        let l = cholesky(&m.cov, 2).unwrap();
        assert!(close(l[0], 2.0) && close(l[2], 0.6) && close(l[3], 0.8));
        let x = from_normals(&m, &l, &[0.5, -1.0]);
        assert!(close(x[0], 1.0 + 2.0 * 0.5) && close(x[1], 2.0 + 0.6 * 0.5 - 0.8));
        assert!(cholesky(&[1.0, 2.0, 2.0, 1.0], 2).is_none(), "not positive definite");
        assert!(close(natural(Transform::Log, 0.0), 1.0));
        assert!(close(natural(Transform::LogitPercent, 0.0), 50.0));
        assert!(close(natural(Transform::LogitShare, 0.0), 0.5));
    }
}
