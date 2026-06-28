/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CellMorphology {
    Basic,
    Producer,
    Consumer,
    Predator,
    Decomposer,
}

/// セルの元素バランスから形態を決定する
pub fn determine_morphology(elements: &[u16; 8]) -> CellMorphology {
    let c = elements[0];
    let n = elements[1];
    let p = elements[2];
    let h = elements[3];
    let o = elements[4];
    let s = elements[5];

    if c > 40000 && n > 30000 {
        CellMorphology::Predator
    } else if h > 40000 && o > 40000 {
        CellMorphology::Producer
    } else if c > 30000 && p > 20000 {
        CellMorphology::Consumer
    } else if s > 30000 && n > 20000 {
        CellMorphology::Decomposer
    } else {
        CellMorphology::Basic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_to_predator() {
        let mut elements = [0u16; 8];
        elements[0] = 45000; // C > 40000
        elements[1] = 35000; // N > 30000

        let morph = determine_morphology(&elements);
        assert_eq!(
            morph,
            CellMorphology::Predator,
            "Should evolve to Predator with high C and N"
        );
    }

    #[test]
    fn test_evolution_to_producer() {
        let mut elements = [0u16; 8];
        elements[3] = 45000; // H > 40000
        elements[4] = 45000; // O > 40000

        let morph = determine_morphology(&elements);
        assert_eq!(
            morph,
            CellMorphology::Producer,
            "Should evolve to Producer with high H and O"
        );
    }
}
