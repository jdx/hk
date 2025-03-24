use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct StepResponse {
    pub files_to_add: Vec<PathBuf>,
}

// impl StepResponse {
//     pub fn extend(&mut self, other: StepResponse) {
//         self.files_to_add.extend(other.files_to_add);
//     }
// }
