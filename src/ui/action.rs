/// What a screen asks the application shell to do.
///
/// Screens handle their own keys and mutate their own state. When a key crosses
/// a screen boundary, the screen returns one of these actions instead of acting
/// on the whole application.
#[derive(Debug, Clone)]
pub enum Action {
    /// Close the application.
    Quit,
    /// Open the named table into the row grid.
    OpenTable(String),
    /// Open the chosen database file into the browse screens.
    PickDatabase(String),
    /// Return from the row grid to the table list.
    BackToList,
    /// Return from the table list to the directory picker.
    BackToPicker,
}
