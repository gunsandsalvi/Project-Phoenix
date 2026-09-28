//! The labour round's arithmetic over a few vacancies and seekers: reach by region, occupation and skill; the logit's
//! shares above the reservation, without replacement; the same for any workers; selection up to the jobs open; and a
//! person's answers, the best-paid accepted, the rest's jobs returned.
#![cfg(test)]

use phx_id::{PartyKey, Slot};
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

use super::{Applicant, Application, Seeker, Standing, Vacancy, answer, search, select};

fn firm(i: u32) -> PartyKey {
    PartyKey::new(2, Slot::new(i))
}

fn vacancy(i: u32, (region, occupation, skill): (u32, u32, u32), wage: f64, open: u32) -> Vacancy {
    Vacancy { employer: firm(i), region, occupation, skill, wage, open }
}

fn seeker(i: u32, skill: u32, reservation: f64) -> Seeker {
    Seeker {
        household: PartyKey::new(1, Slot::new(i)),
        person: 0,
        subject: u64::from(i),
        region: 0,
        occupation: 3,
        skill,
        experience: i % 7,
        reservation,
    }
}

fn draws(subject: u64) -> Draws {
    Draws::new(stream_key(Seed::new(1), "LAB.match_taste"), Subject::new(SubjectTag::Party, subject), 5, 7)
}

#[test]
fn reach_is_the_region_occupation_and_skill() {
    let v = [
        vacancy(0, (0, 3, 2), 1_000.0, 1),
        vacancy(1, (0, 3, 1), 1_000.0, 1),
        vacancy(2, (0, 4, 1), 1_000.0, 1),
        vacancy(3, (1, 3, 1), 1_000.0, 1),
        vacancy(4, (0, 3, 1), 1_000.0, 0),
        vacancy(5, (0, 3, 3), 1_000.0, 2),
    ];
    let st = Standing::new(&v);
    assert_eq!(st.in_reach(&v, &seeker(0, 2, 0.0)), &[1, 0], "its group's, least skill first; none closed");
    assert!(st.in_reach(&v, &seeker(0, 0, 0.0)).is_empty(), "no job asking more than it has");
    assert!(st.in_reach(&v, &Seeker { occupation: 9, ..seeker(0, 9, 0.0) }).is_empty());
}

#[test]
fn applications_follow_the_logit_above_the_reservation() {
    let v = [
        vacancy(0, (0, 3, 1), 900.0, 1),
        vacancy(1, (0, 3, 1), 1_200.0, 1),
        vacancy(2, (0, 3, 1), 1_500.0, 1),
        vacancy(3, (0, 3, 1), 2_000.0, 1),
    ];
    let st = Standing::new(&v);
    let n = 30_000_u32;
    let seekers: Vec<Seeker> = (0..n).map(|i| seeker(i, 1, 1_000.0)).collect();
    let weight = 2.0;
    let apps = search(None, (&v, &st), &seekers, (weight, 1.0), &draws);
    assert_eq!(apps.len(), 30_000, "one application a day each");
    let pulls: Vec<f64> = [1_200.0_f64, 1_500.0, 2_000.0].iter().map(|w| w.powf(weight)).collect();
    let total: f64 = pulls.iter().sum();
    for (k, p) in pulls.iter().enumerate() {
        let got = apps.iter().filter(|a| a.vacancy == u32::try_from(k).unwrap() + 1).count();
        let share = f64::from(u32::try_from(got).unwrap()) / f64::from(n);
        assert!((share - p / total).abs() < 0.01, "vacancy {}: {share} against {}", k + 1, p / total);
    }
    assert!(apps.iter().all(|a| a.vacancy != 0), "none to a wage below the reservation");
    let many = search(None, (&v, &st), &seekers[..50], (weight, 5.0), &draws);
    for s in 0..50 {
        let mine: Vec<u32> = many.iter().filter(|a| a.seeker.subject == s).map(|a| a.vacancy).collect();
        let mut distinct = mine.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!((mine.len(), distinct.len()), (3, 3), "without replacement, as many as it can reach");
    }
    let pool = phx_exec::Pool::new(&phx_exec::PoolSpec::unpinned(4)).unwrap();
    assert_eq!(
        search(Some(&pool), (&v, &st), &seekers, (weight, 1.4), &draws),
        search(None, (&v, &st), &seekers, (weight, 1.4), &draws)
    );
}

#[test]
fn employers_select_up_to_their_jobs() {
    let mut v = [vacancy(0, (0, 3, 1), 1_000.0, 2), vacancy(1, (0, 3, 1), 1_000.0, 1)];
    let apps: Vec<Application> =
        (0..5).map(|i| Application { vacancy: i % 2, seeker: seeker(i, 1 + i, 0.0) }).collect();
    let lots =
        |v: u32| Draws::new(stream_key(Seed::new(1), "LAB.lot"), Subject::new(SubjectTag::Market, u64::from(v)), 5, 7);
    let by_skill = |a: &[Applicant], open: u32| {
        let mut order: Vec<u32> = (0..u32::try_from(a.len()).unwrap()).collect();
        order.sort_by_key(|k| std::cmp::Reverse(a[usize::try_from(*k).unwrap()].skill));
        order.truncate(usize::try_from(open).unwrap());
        order
    };
    let met = |a: &Application| a.seeker.subject != 4;
    let offers = select(&mut v, &apps, met, (lots, by_skill));
    let got: Vec<(u32, u64)> = offers.iter().map(|o| (o.vacancy, o.seeker.subject)).collect();
    assert_eq!(got, vec![(0, 2), (0, 0), (1, 3)], "the most skilled met, up to the jobs open; the unmet not at all");
    assert_eq!((v[0].open, v[1].open), (0, 0));
}

#[test]
fn a_person_takes_the_best_paid_offer_it_accepts() {
    let mut v =
        [vacancy(0, (0, 3, 1), 1_000.0, 0), vacancy(1, (0, 3, 1), 1_400.0, 0), vacancy(2, (0, 3, 1), 1_800.0, 0)];
    let s = seeker(0, 1, 1_100.0);
    let offers: Vec<Application> = (0..3).map(|k| Application { vacancy: k, seeker: s }).collect();
    let hires = answer(&mut v, &offers, |_, v| v.wage < 1_500.0);
    assert_eq!(hires.iter().map(|h| h.vacancy).collect::<Vec<_>>(), vec![1], "the best-paid it accepts");
    assert_eq!(v.map(|x| x.open), [1, 0, 1], "the others' jobs return");
    let none = answer(&mut v, &offers, |_, _| false);
    assert!(none.is_empty());
    assert_eq!(v.map(|x| x.open), [2, 1, 2]);
}
