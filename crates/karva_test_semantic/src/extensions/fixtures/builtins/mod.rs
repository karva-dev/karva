pub use mock_env::MockEnv;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator};

use crate::extensions::fixtures::NormalizedFixture;

mod caplog;
mod capsys;
mod mock_env;
mod recwarn;
mod temp_path;

use capsys::{
    create_capfd_fixture, create_capfdbinary_fixture, create_capsysbinary_fixture,
    is_capfd_fixture_name, is_capfdbinary_fixture_name, is_capsysbinary_fixture_name,
};

pub fn get_builtin_fixture(py: Python<'_>, fixture_name: &str) -> Option<NormalizedFixture> {
    match fixture_name {
        _ if temp_path::is_temp_path_fixture_name(fixture_name) => {
            if let Some(path_obj) = temp_path::create_temp_dir_fixture(py) {
                return Some(NormalizedFixture::built_in(
                    fixture_name.to_string(),
                    path_obj,
                ));
            }
        }
        _ if temp_path::is_tmpdir_fixture_name(fixture_name) => {
            if let Some(path_obj) = temp_path::create_tmpdir_fixture(py) {
                return Some(NormalizedFixture::built_in(
                    fixture_name.to_string(),
                    path_obj,
                ));
            }
        }
        _ if temp_path::is_tmp_path_factory_fixture_name(fixture_name) => {
            if let Some(factory) = temp_path::create_tmp_path_factory_fixture(py) {
                return Some(NormalizedFixture::built_in(
                    fixture_name.to_string(),
                    factory,
                ));
            }
        }
        _ if temp_path::is_tmpdir_factory_fixture_name(fixture_name) => {
            if let Some(factory) = temp_path::create_tmpdir_factory_fixture(py) {
                return Some(NormalizedFixture::built_in(
                    fixture_name.to_string(),
                    factory,
                ));
            }
        }
        _ if mock_env::is_mock_env_fixture_name(fixture_name) => {
            if let Some((mock_instance, finalizer)) = mock_env::create_mock_env_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    mock_instance,
                    finalizer,
                ));
            }
        }
        _ if caplog::is_caplog_fixture_name(fixture_name) => {
            if let Some((caplog_instance, finalizer)) = caplog::create_caplog_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    caplog_instance,
                    finalizer,
                ));
            }
        }
        _ if capsys::is_capsys_fixture_name(fixture_name) => {
            if let Some((capsys_instance, finalizer)) = capsys::create_capsys_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    capsys_instance,
                    finalizer,
                ));
            }
        }
        _ if is_capfd_fixture_name(fixture_name) => {
            if let Some((capfd_instance, finalizer)) = create_capfd_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    capfd_instance,
                    finalizer,
                ));
            }
        }
        _ if is_capsysbinary_fixture_name(fixture_name) => {
            if let Some((capsysbinary_instance, finalizer)) = create_capsysbinary_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    capsysbinary_instance,
                    finalizer,
                ));
            }
        }
        _ if is_capfdbinary_fixture_name(fixture_name) => {
            if let Some((capfdbinary_instance, finalizer)) = create_capfdbinary_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    capfdbinary_instance,
                    finalizer,
                ));
            }
        }
        _ if recwarn::is_recwarn_fixture_name(fixture_name) => {
            if let Some((recwarn_instance, finalizer)) = recwarn::create_recwarn_fixture(py) {
                return Some(NormalizedFixture::built_in_with_finalizer(
                    fixture_name.to_string(),
                    recwarn_instance,
                    finalizer,
                ));
            }
        }
        _ => {}
    }

    None
}

/// Only used for builtin fixtures where we need to synthesize a fixture finalizer
pub fn create_fixture_with_finalizer<'py>(
    py: Python<'py>,
    fixture_return_value: &Py<PyAny>,
    finalizer_function: &Py<PyAny>,
) -> PyResult<Bound<'py, PyIterator>> {
    let code = r"
def _builtin_finalizer(value, finalizer):
    yield value
    finalizer()
    ";

    let locals = PyDict::new(py);

    py.run(
        &std::ffi::CString::new(code).expect("fixture code contains null byte"),
        None,
        Some(&locals),
    )?;

    let generator_function = locals
        .get_item("_builtin_finalizer")?
        .expect("To find generator the function");

    let iterator = generator_function.call1((fixture_return_value, finalizer_function))?;

    let iterator = iterator.cast_into::<PyIterator>()?;

    Ok(iterator)
}
