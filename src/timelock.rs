use crate::{Difficulty, Error, Result, Wide, TL_BYTES};
use crypto_bigint::modular::{FixedMontyForm, FixedMontyParams};
use crypto_bigint::{Odd, U2048};

/// Solves the time-lock puzzle: `base^(2^difficulty) mod modulus` by squaring `difficulty` times
///
/// Use
/// [`Solver::solve`] for a one-shot blocking solve or
/// [`Solver::new`] & [`Solver::step`] to run it in bounded chunks while reporting progress.
pub struct Solver {
    current: FixedMontyForm<{ U2048::LIMBS }>,
    done: u64,
    total: u64,
}

impl Solver {
    pub fn new(modulus: &Wide, base: &Wide, difficulty: Difficulty) -> Result<Self> {
        let modulus = U2048::from_be_slice(modulus.as_bytes());
        let modulus = Odd::new(modulus).into_option().ok_or(Error::BadModulus)?;
        let params = FixedMontyParams::new(modulus);
        let current = FixedMontyForm::new(&U2048::from_be_slice(base.as_bytes()), &params);
        Ok(Self { current, done: 0, total: difficulty.0 })
    }

    /// One-shot blocking solve
    pub fn solve(modulus: &Wide, base: &Wide, difficulty: Difficulty) -> Result<Wide> {
        let mut solver = Self::new(modulus, base, difficulty)?;
        solver.step(difficulty.0);
        Ok(solver.answer().expect("is_done true after {difficulty} steps"))
    }

    /// Squares up to `steps` more times. Returns `true` once the chain is done.
    #[inline(always)]
    pub fn step(&mut self, steps: u64) -> bool {
        let n = steps.min(self.total - self.done);
        for _ in 0..n {
            self.current = self.current.square();
        }
        self.done += n;
        self.is_done()
    }

    #[must_use]
    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.done == self.total
    }

    /// Progress report, 0.0 - 1.0
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.total == 0 { 1.0 } else { self.done as f64 / self.total as f64 }
    }

    #[must_use]
    pub fn answer(&self) -> Option<Wide> {
        if !self.is_done() {
            return None
        }

        let bytes: [u8; TL_BYTES] = self.current
            .retrieve()
            .to_be_bytes()
            .into();
        Some(Wide::from_bytes(bytes))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod server {
    use crate::{Difficulty, Wide, TL_BYTES};
    use crypto_bigint::modular::{FixedMontyForm, FixedMontyParams};
    use crypto_bigint::{Odd, U1024};
    use num_bigint::{BigInt, BigUint, RandBigInt};
    use num_integer::Integer;
    use num_prime::RandPrime;
    use num_traits::One;
    use rand::rngs::OsRng;
    use std::collections::HashMap;
    use std::hash::Hash;
    use std::time::{Duration, Instant};

    /// A 1024-bit trapdoor prime carrying precomputed Montgomery parameters so
    /// each `base^e mod p` is a fixed-size, allocation-free, constant-time modexp
    struct Prime {
        value: BigUint,
        params: FixedMontyParams<{ U1024::LIMBS }>,
    }

    impl Prime {
        fn new(value: BigUint) -> Self {
            let modulus = Odd::new(biguint_to_u1024(&value))
                .into_option()
                .expect("a trapdoor prime is odd");
            Self { params: FixedMontyParams::new(modulus), value }
        }

        fn modexp(&self, base: &BigUint, exponent: &BigUint) -> BigUint {
            let base = FixedMontyForm::new(&biguint_to_u1024(&(base % &self.value)), &self.params);
            u1024_to_biguint(&base.pow(&biguint_to_u1024(exponent)).retrieve())
        }
    }

    fn biguint_to_u1024(value: &BigUint) -> U1024 {
        let be = value.to_bytes_be();
        assert!(be.len() <= 128, "value wider than 1024 bits");
        let mut bytes = [0u8; 128];
        bytes[128 - be.len()..].copy_from_slice(&be);
        U1024::from_be_slice(&bytes)
    }

    fn u1024_to_biguint(value: &U1024) -> BigUint {
        BigUint::from_bytes_be(value.to_be_bytes().as_ref())
    }

    /// Our long-lived secret.
    /// Generate once and reuse the modulus across challenges.
    /// `p`, `q`, and `qinv` can never be allowed to leave the server.
    pub struct Trapdoor {
        n: BigUint,
        p: Prime,
        q: Prime,
        qinv: BigUint,
    }

    impl Trapdoor {
        #[must_use]
        pub fn generate(modulus_bits: usize) -> Self {
            assert!(
                (1024..=2048).contains(&modulus_bits) && modulus_bits.is_multiple_of(2),
                "modulus_bits must be even and within 1024..=2048"
            );
            let prime_bits = modulus_bits / 2;
            let mut rng = OsRng;
            let p: BigUint = rng.gen_prime_exact(prime_bits, None);
            let mut q: BigUint = rng.gen_prime_exact(prime_bits, None);
            while q == p {
                q = rng.gen_prime_exact(prime_bits, None);
            }
            let n = &p * &q;
            let qinv = Self::modinv(&q, &p);
            Self { n, p: Prime::new(p), q: Prime::new(q), qinv }
        }

        #[must_use]
        pub fn modulus(&self) -> Wide {
            Wide::from_biguint(&self.n)
        }

        fn answer(&self, base: &BigUint, difficulty: Difficulty) -> BigUint {
            let two = BigUint::from(2u32);
            let t = BigUint::from(difficulty.0);
            let exp_p = two.modpow(&t, &(&self.p.value - BigUint::one()));
            let exp_q = two.modpow(&t, &(&self.q.value - BigUint::one()));
            let y_p = self.p.modexp(base, &exp_p);
            let y_q = self.q.modexp(base, &exp_q);
            self.garner(&y_p, &y_q)
        }

        fn garner(&self, y_p: &BigUint, y_q: &BigUint) -> BigUint {
            let p = BigInt::from(self.p.value.clone());
            let diff = (BigInt::from(y_p.clone()) - BigInt::from(y_q.clone())).mod_floor(&p);
            let h = (diff * BigInt::from(self.qinv.clone()))
                .mod_floor(&p)
                .to_biguint()
                .expect("mod_floor yields a value in [0, p)");
            y_q + &self.q.value * h
        }

        fn modinv(a: &BigUint, modulus: &BigUint) -> BigUint {
            let m = BigInt::from(modulus.clone());
            BigInt::from(a.clone())
                .extended_gcd(&m)
                .x
                .mod_floor(&m)
                .to_biguint()
                .expect("mod_floor yields a value in [0, modulus)")
        }

        fn random_base(&self) -> BigUint {
            let mut rng = OsRng;
            let two = BigUint::from(2u32);
            loop {
                let candidate = rng.gen_biguint_below(&self.n);
                if candidate > two && candidate.gcd(&self.n).is_one() {
                    return candidate;
                }
            }
        }

        #[must_use]
        pub fn new_challenge(&self, difficulty: Difficulty) -> (Challenge, Wide) {
            let base = self.random_base();
            let answer = self.answer(&base, difficulty);
            let challenge = Challenge {
                modulus: self.modulus(),
                base: Wide::from_biguint(&base),
                difficulty,
            };
            (challenge, Wide::from_biguint(&answer))
        }
    }

    impl Wide {
        fn from_biguint(value: &BigUint) -> Self {
            let be = value.to_bytes_be();
            assert!(be.len() <= TL_BYTES, "value wider than the fixed 2048-bit width");
            let mut bytes = [0u8; TL_BYTES];
            bytes[TL_BYTES - be.len()..].copy_from_slice(&be);
            Self::from_bytes(bytes)
        }
    }

    #[derive(Clone, Debug)]
    pub struct Challenge {
        pub modulus: Wide,
        pub base: Wide,
        pub difficulty: Difficulty,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Verify {
        Accepted,
        Wrong,
        UnknownOrExpired,
    }

    struct Outstanding {
        answer: Wide,
        expires: Instant,
    }

    pub struct ChallengeStore<K> {
        trapdoor: Trapdoor,
        issued: HashMap<K, Outstanding>,
        ttl: Duration,
    }

    impl<K: Hash + Eq> ChallengeStore<K> {
        #[must_use]
        pub fn new(trapdoor: Trapdoor, ttl: Duration) -> Self {
            Self { trapdoor, issued: HashMap::new(), ttl }
        }

        #[must_use]
        pub fn trapdoor(&self) -> &Trapdoor {
            &self.trapdoor
        }

        /// Issue a challenge for `key`. Replaces any existing entry for it
        pub fn issue(&mut self, key: K, difficulty: Difficulty) -> Challenge {
            let (challenge, answer) = self.trapdoor.new_challenge(difficulty);
            self.issued
                .insert(key, Outstanding { answer, expires: Instant::now() + self.ttl });
            challenge
        }

        /// Check if `key` has an unexpired challenge
        #[must_use]
        pub fn has_live(&self, key: &K) -> bool {
            self.issued.get(key).is_some_and(|entry| entry.expires > Instant::now())
        }

        /// Correct answers are immediately consumed
        pub fn verify(&mut self, key: &K, submitted: &Wide) -> Verify {
            let Some(entry) = self.issued.get(key) else {
                return Verify::UnknownOrExpired;
            };
            if Instant::now() > entry.expires {
                self.issued.remove(key);
                return Verify::UnknownOrExpired;
            }
            if *submitted == entry.answer {
                self.issued.remove(key);
                Verify::Accepted
            } else {
                Verify::Wrong
            }
        }

        pub fn gc(&mut self) {
            let now = Instant::now();
            self.issued.retain(|_, entry| entry.expires > now);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::Solver;
    use super::server::{ChallengeStore, Trapdoor, Verify};
    use crate::Difficulty;
    use std::net::Ipv4Addr;
    use std::time::Duration;

    #[test]
    fn client_solution_matches_trapdoor_answer() {
        let trapdoor = Trapdoor::generate(1024);
        let (challenge, expected) = trapdoor.new_challenge(Difficulty(4096));

        let solved = Solver::solve(&challenge.modulus, &challenge.base, challenge.difficulty).unwrap();

        assert_eq!(solved, expected);
    }

    #[test]
    fn store_keyed_by_ip_accepts_correct_and_rejects_replay() {
        let mut store: ChallengeStore<Ipv4Addr> =
            ChallengeStore::new(Trapdoor::generate(1024), Duration::from_secs(60));
        let client = Ipv4Addr::new(203, 0, 113, 7);

        let challenge = store.issue(client, Difficulty(1024));
        let answer = Solver::solve(&challenge.modulus, &challenge.base, challenge.difficulty).unwrap();

        assert!(store.has_live(&client));
        assert_eq!(store.verify(&client, &answer), Verify::Accepted);
        // Accepted challenges are consumed. If the entry is gone, a resubmit gets UnknownOrExpired
        assert!(!store.has_live(&client));
        assert_eq!(store.verify(&client, &answer), Verify::UnknownOrExpired);
    }

    #[test]
    fn reissue_for_same_key_replaces_the_outstanding_challenge() {
        let mut store = ChallengeStore::new(Trapdoor::generate(1024), Duration::from_secs(60));

        let first = store.issue(7, Difficulty(512));
        let first_answer = Solver::solve(&first.modulus, &first.base, first.difficulty).unwrap();

        let second = store.issue(7, Difficulty(512));
        let second_answer = Solver::solve(&second.modulus, &second.base, second.difficulty).unwrap();

        assert_eq!(store.verify(&7, &first_answer), Verify::Wrong);
        assert_eq!(store.verify(&7, &second_answer), Verify::Accepted);
    }
}