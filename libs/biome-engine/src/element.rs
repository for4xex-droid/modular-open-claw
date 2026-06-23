use crate::grid::BiomeCell;

// 元素インデックスの定数定義
pub const ELEMENT_C: usize = 0;
pub const ELEMENT_N: usize = 1;
pub const ELEMENT_P: usize = 2;
pub const ELEMENT_H: usize = 3;
pub const ELEMENT_O: usize = 4;
pub const ELEMENT_S: usize = 5;
pub const ELEMENT_FE: usize = 6;
pub const ELEMENT_SI: usize = 7;

// 反応閾値の定数定義
pub const METABOLISM_THRESHOLD: u16 = 100;
pub const RESPIRATION_THRESHOLD: u16 = 100;
pub const ENERGY_BURST_THRESHOLD: u16 = 200;
pub const DEFENSE_HARDENING_THRESHOLD: u16 = 100;
pub const SUPERCONDUCTIVITY_THRESHOLD: u16 = 200;
pub const VITAL_CATALYSIS_THRESHOLD: u16 = 150;

/// 元素反応を処理する (質量保存: 消費される質量の総和 = 生成される質量の総和)
pub fn react_elements(cell: &mut BiomeCell) {
    // 1. 代謝反応: C (100) + H (100) -> N (200)
    let c = cell.elements[ELEMENT_C];
    let h = cell.elements[ELEMENT_H];
    if c >= METABOLISM_THRESHOLD && h >= METABOLISM_THRESHOLD {
        let reactions = (c / METABOLISM_THRESHOLD).min(h / METABOLISM_THRESHOLD);
        cell.elements[ELEMENT_C] -= reactions * METABOLISM_THRESHOLD;
        cell.elements[ELEMENT_H] -= reactions * METABOLISM_THRESHOLD;
        cell.elements[ELEMENT_N] =
            cell.elements[ELEMENT_N].saturating_add(reactions * (METABOLISM_THRESHOLD * 2));
    }

    // 2. 呼吸反応: O (100) + S (100) -> P (200)
    let o = cell.elements[ELEMENT_O];
    let s = cell.elements[ELEMENT_S];
    if o >= RESPIRATION_THRESHOLD && s >= RESPIRATION_THRESHOLD {
        let reactions = (o / RESPIRATION_THRESHOLD).min(s / RESPIRATION_THRESHOLD);
        cell.elements[ELEMENT_O] -= reactions * RESPIRATION_THRESHOLD;
        cell.elements[ELEMENT_S] -= reactions * RESPIRATION_THRESHOLD;
        cell.elements[ELEMENT_P] =
            cell.elements[ELEMENT_P].saturating_add(reactions * (RESPIRATION_THRESHOLD * 2));
    }

    // 3. エネルギーバースト: N(200) + P(200) -> C(200) + H(200)
    let n = cell.elements[ELEMENT_N];
    let p = cell.elements[ELEMENT_P];
    if n >= ENERGY_BURST_THRESHOLD && p >= ENERGY_BURST_THRESHOLD {
        let reactions = (n / ENERGY_BURST_THRESHOLD).min(p / ENERGY_BURST_THRESHOLD);
        cell.elements[ELEMENT_N] -= reactions * ENERGY_BURST_THRESHOLD;
        cell.elements[ELEMENT_P] -= reactions * ENERGY_BURST_THRESHOLD;
        cell.elements[ELEMENT_C] =
            cell.elements[ELEMENT_C].saturating_add(reactions * ENERGY_BURST_THRESHOLD);
        cell.elements[ELEMENT_H] =
            cell.elements[ELEMENT_H].saturating_add(reactions * ENERGY_BURST_THRESHOLD);
    }

    // 4. 防御硬化: Fe(100) + O(100) -> Si(200)
    let fe = cell.elements[ELEMENT_FE];
    let o_val = cell.elements[ELEMENT_O];
    if fe >= DEFENSE_HARDENING_THRESHOLD && o_val >= DEFENSE_HARDENING_THRESHOLD {
        let reactions = (fe / DEFENSE_HARDENING_THRESHOLD).min(o_val / DEFENSE_HARDENING_THRESHOLD);
        cell.elements[ELEMENT_FE] -= reactions * DEFENSE_HARDENING_THRESHOLD;
        cell.elements[ELEMENT_O] -= reactions * DEFENSE_HARDENING_THRESHOLD;
        cell.elements[ELEMENT_SI] =
            cell.elements[ELEMENT_SI].saturating_add(reactions * (DEFENSE_HARDENING_THRESHOLD * 2));
    }

    // 5. 超伝導合成: Si(200) + Fe(200) -> C(100) + N(100) + P(100) + H(100)
    let si = cell.elements[ELEMENT_SI];
    let fe_val = cell.elements[ELEMENT_FE];
    if si >= SUPERCONDUCTIVITY_THRESHOLD && fe_val >= SUPERCONDUCTIVITY_THRESHOLD {
        let reactions =
            (si / SUPERCONDUCTIVITY_THRESHOLD).min(fe_val / SUPERCONDUCTIVITY_THRESHOLD);
        cell.elements[ELEMENT_SI] -= reactions * SUPERCONDUCTIVITY_THRESHOLD;
        cell.elements[ELEMENT_FE] -= reactions * SUPERCONDUCTIVITY_THRESHOLD;
        cell.elements[ELEMENT_C] =
            cell.elements[ELEMENT_C].saturating_add(reactions * (SUPERCONDUCTIVITY_THRESHOLD / 2));
        cell.elements[ELEMENT_N] =
            cell.elements[ELEMENT_N].saturating_add(reactions * (SUPERCONDUCTIVITY_THRESHOLD / 2));
        cell.elements[ELEMENT_P] =
            cell.elements[ELEMENT_P].saturating_add(reactions * (SUPERCONDUCTIVITY_THRESHOLD / 2));
        cell.elements[ELEMENT_H] =
            cell.elements[ELEMENT_H].saturating_add(reactions * (SUPERCONDUCTIVITY_THRESHOLD / 2));
    }

    // 6. 生命活性化: S(150) + H(150) -> O(300)
    let s_val = cell.elements[ELEMENT_S];
    let h_val = cell.elements[ELEMENT_H];
    if s_val >= VITAL_CATALYSIS_THRESHOLD && h_val >= VITAL_CATALYSIS_THRESHOLD {
        let reactions = (s_val / VITAL_CATALYSIS_THRESHOLD).min(h_val / VITAL_CATALYSIS_THRESHOLD);
        cell.elements[ELEMENT_S] -= reactions * VITAL_CATALYSIS_THRESHOLD;
        cell.elements[ELEMENT_H] -= reactions * VITAL_CATALYSIS_THRESHOLD;
        cell.elements[ELEMENT_O] =
            cell.elements[ELEMENT_O].saturating_add(reactions * (VITAL_CATALYSIS_THRESHOLD * 2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::CellGenome;

    #[test]
    fn test_element_reaction_produces_organic_matter() {
        let mut cell = BiomeCell {
            active: true,
            elements: [
                1000, // C
                500,  // N
                0,    // P
                1000, // H
                1000, // O
                0,    // S
                0,    // Fe
                0,    // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        react_elements(&mut cell);

        assert!(
            cell.elements[0] < 1000,
            "Carbon should be consumed in metabolic reaction"
        );
        assert_eq!(
            cell.elements[1], 2500,
            "Nitrogen should increase by converted mass"
        );
    }

    #[test]
    fn test_mass_conservation_during_reaction() {
        let mut cell = BiomeCell {
            active: true,
            elements: [1000, 500, 200, 1000, 1000, 500, 0, 0],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        let initial_mass: u32 = cell.elements.iter().map(|&x| x as u32).sum();

        react_elements(&mut cell);

        let final_mass: u32 = cell.elements.iter().map(|&x| x as u32).sum();

        assert_eq!(
            initial_mass, final_mass,
            "Total mass must be conserved during reactions"
        );
    }

    #[test]
    fn test_energy_burst_reaction() {
        let mut cell = BiomeCell {
            active: true,
            elements: [
                0,   // C
                400, // N
                400, // P
                0,   // H
                0,   // O
                0,   // S
                0,   // Fe
                0,   // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        react_elements(&mut cell);

        // 反応前: N:400, P:400, 他0
        // エネルギーバースト: N(200) + P(200) -> C(200) + H(200)
        // 2回反応が起こるはず
        // 反応後: N:0, P:0, C:400, H:400
        assert_eq!(cell.elements[1], 0, "Nitrogen should be consumed");
        assert_eq!(cell.elements[2], 0, "Phosphorus should be consumed");
        assert_eq!(cell.elements[0], 400, "Carbon should be generated");
        assert_eq!(cell.elements[3], 400, "Hydrogen should be generated");
    }

    #[test]
    fn test_defense_hardening_reaction() {
        let mut cell = BiomeCell {
            active: true,
            elements: [
                0,   // C
                0,   // N
                0,   // P
                0,   // H
                200, // O
                0,   // S
                200, // Fe
                0,   // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        react_elements(&mut cell);

        // 反応前: O:200, Fe:200, 他0
        // 防御硬化: Fe(100) + O(100) -> Si(200)
        // 2回反応が起こるはず
        // 反応後: O:0, Fe:0, Si:400
        assert_eq!(cell.elements[6], 0, "Iron should be consumed");
        assert_eq!(cell.elements[4], 0, "Oxygen should be consumed");
        assert_eq!(cell.elements[7], 400, "Silicon should be generated");
    }

    #[test]
    fn test_superconductivity_reaction() {
        let mut cell = BiomeCell {
            active: true,
            elements: [
                0,   // C
                0,   // N
                0,   // P
                0,   // H
                0,   // O
                0,   // S
                200, // Fe
                200, // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        react_elements(&mut cell);

        assert_eq!(cell.elements[6], 0, "Iron should be consumed");
        assert_eq!(cell.elements[7], 0, "Silicon should be consumed");
        assert_eq!(cell.elements[0], 100, "Carbon should be generated");
        assert_eq!(cell.elements[1], 100, "Nitrogen should be generated");
        assert_eq!(cell.elements[2], 100, "Phosphorus should be generated");
        assert_eq!(cell.elements[3], 100, "Hydrogen should be generated");
    }

    #[test]
    fn test_vital_catalysis_reaction() {
        let mut cell = BiomeCell {
            active: true,
            elements: [
                0,   // C
                0,   // N
                0,   // P
                300, // H
                0,   // O
                300, // S
                0,   // Fe
                0,   // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        react_elements(&mut cell);

        assert_eq!(cell.elements[3], 0, "Hydrogen should be consumed");
        assert_eq!(cell.elements[5], 0, "Sulfur should be consumed");
        assert_eq!(cell.elements[4], 600, "Oxygen should be generated");
    }

    #[test]
    fn test_chained_reactions_sequence() {
        // 初期状態: Fe: 400, O: 200, 他は 0
        // 1. 防御硬化 (Fe + O -> Si):
        //    Fe(100) + O(100) -> Si(200) が 2回起きる。
        //    消費: Fe: -200, O: -200
        //    生成: Si: +400
        //    中間状態: Fe: 200, O: 0, Si: 400
        // 2. 超伝導合成 (Si + Fe -> C + N + P + H):
        //    Si(200) + Fe(200) -> C(100) + N(100) + P(100) + H(100) が 1回起きる。
        //    消費: Si: -200, Fe: -200
        //    生成: C: +100, N: +100, P: +100, H: +100
        //    最終状態: Fe: 0, O: 0, Si: 200, C: 100, N: 100, P: 100, H: 100
        let mut cell = BiomeCell {
            active: true,
            elements: [
                0,   // C
                0,   // N
                0,   // P
                0,   // H
                200, // O
                0,   // S
                400, // Fe
                0,   // Si
            ],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: crate::evolution::CellMorphology::Basic,
        };

        let initial_mass: u32 = cell.elements.iter().map(|&x| x as u32).sum();

        react_elements(&mut cell);

        let final_mass: u32 = cell.elements.iter().map(|&x| x as u32).sum();

        assert_eq!(
            initial_mass, final_mass,
            "Mass must be conserved in chained reactions"
        );
        assert_eq!(cell.elements[6], 0, "Iron should be fully consumed");
        assert_eq!(cell.elements[4], 0, "Oxygen should be fully consumed");
        assert_eq!(cell.elements[7], 200, "Remaining Silicon should be 200");
        assert_eq!(cell.elements[0], 100, "C should be 100");
        assert_eq!(cell.elements[1], 100, "N should be 100");
        assert_eq!(cell.elements[2], 100, "P should be 100");
        assert_eq!(cell.elements[3], 100, "H should be 100");
    }
}
