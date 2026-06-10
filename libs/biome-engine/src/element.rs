use crate::grid::BiomeCell;

/// 元素反応を処理する (質量保存: 消費される質量の総和 = 生成される質量の総和)
pub fn react_elements(cell: &mut BiomeCell) {
    // 1. 代謝反応: C (100) + H (100) -> N (200)
    let c = cell.elements[0];
    let h = cell.elements[3];
    if c >= 100 && h >= 100 {
        let reactions = (c / 100).min(h / 100);
        cell.elements[0] -= reactions * 100;
        cell.elements[3] -= reactions * 100;
        cell.elements[1] = cell.elements[1].saturating_add(reactions * 200);
    }

    // 2. 呼吸反応: O (100) + S (100) -> P (200)
    let o = cell.elements[4];
    let s = cell.elements[5];
    if o >= 100 && s >= 100 {
        let reactions = (o / 100).min(s / 100);
        cell.elements[4] -= reactions * 100;
        cell.elements[5] -= reactions * 100;
        cell.elements[2] = cell.elements[2].saturating_add(reactions * 200);
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
}
