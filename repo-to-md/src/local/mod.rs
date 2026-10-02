mod assets;
mod handlers;
pub mod highlighting;
mod refspec;
mod server;
mod state;

pub use refspec::{RefSpec, detect_base_branch};
pub use server::{BoundServer, bind_server, bind_server_with_initial_comments};
pub use state::CommentsFile;
