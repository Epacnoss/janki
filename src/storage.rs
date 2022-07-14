use crate::{game::AnkiDB};
use std::error::Error;

///Trait for a place to store the database
pub trait Storage {
    ///An associated type for errors that come from the functions. Must implement [`std::error::Error`]
    type ErrorType: Error;

    ///Read the database into memory, and return an [`AnkiDB`] or an Error using [`Self::ErrorType`]
    fn read_db(&self) -> Result<AnkiDB, Self::ErrorType>;
    ///Writes an [`AnkiDB`] to Storage, and returns a [`Result::Err`] on failure
    fn write_db(&mut self, db: &AnkiDB) -> Result<(), Self::ErrorType>;
    ///Exits the application - not always necessary, as things like files can be automatically dropped
    fn exit_application(&mut self) {}
}
