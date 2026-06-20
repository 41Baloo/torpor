use crate::{Difficulty, Wide, timelock};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Solver(timelock::Solver);

#[wasm_bindgen]
impl Solver {
    #[wasm_bindgen(constructor)]
    pub fn new(modulus_hex: &str, base_hex: &str, difficulty: u64) -> Result<Solver, JsError> {
        let modulus = Wide::from_hex(modulus_hex)?;
        let base = Wide::from_hex(base_hex)?;
        Ok(Solver(timelock::Solver::new(&modulus, &base, Difficulty(difficulty))?))
    }

    /// Square up to `steps` more times. Returns `true` once the chain is done
    pub fn step(&mut self, steps: u64) -> bool {
        self.0.step(steps)
    }

    #[wasm_bindgen(getter)]
    pub fn done(&self) -> bool {
        self.0.is_done()
    }

    #[wasm_bindgen(getter)]
    pub fn progress(&self) -> f64 {
        self.0.progress()
    }

    #[wasm_bindgen(getter, js_name = answerHex)]
    pub fn answer_hex(&self) -> Option<String> {
        self.0.answer().map(|answer| answer.to_hex())
    }
}
