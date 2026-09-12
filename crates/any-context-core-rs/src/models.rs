use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPayload {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub text: String,
    #[pyo3(get, set)]
    pub file_name: String,
    #[pyo3(get, set)]
    pub file_path: String,
    #[pyo3(get, set)]
    pub header_path: Option<String>,
    #[pyo3(get, set)]
    pub start_line: usize,
    #[pyo3(get, set)]
    pub end_line: usize,
    #[pyo3(get, set)]
    pub content_type: String,
    #[pyo3(get, set)]
    pub chunk_index: usize,
}

#[pymethods]
impl ChunkPayload {
    #[new]
    #[pyo3(signature = (id, text, file_name, file_path, header_path=None, start_line=0, end_line=0, content_type="markdown".to_string(), chunk_index=0))]
    pub fn new(
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        header_path: Option<String>,
        start_line: usize,
        end_line: usize,
        content_type: String,
        chunk_index: usize,
    ) -> Self {
        Self {
            id,
            text,
            file_name,
            file_path,
            header_path,
            start_line,
            end_line,
            content_type,
            chunk_index,
        }
    }

    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("text", &self.text)?;
        dict.set_item("file_name", &self.file_name)?;
        dict.set_item("file_path", &self.file_path)?;
        dict.set_item("header_path", &self.header_path)?;
        dict.set_item("start_line", self.start_line)?;
        dict.set_item("end_line", self.end_line)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("chunk_index", self.chunk_index)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ChunkPayload(file='{}', lines={}-{}, header={:?}, text_len={})",
            self.file_name,
            self.start_line,
            self.end_line,
            self.header_path,
            self.text.len()
        )
    }
}
