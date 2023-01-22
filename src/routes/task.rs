use crate::fairings::MinneDatabaseConnection;
use crate::guards::AuthenticatedUser;
use crate::schema::tasks;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;

#[derive(Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub title: String,
    pub owner: i32,
}

#[derive(Deserialize)]
pub struct NewTaskData {
    /// The title for the new task.
    pub title: String,
}

#[post("/task/new", data = "<new_task_data>")]
pub async fn add_new_task(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    new_task_data: Json<NewTaskData>,
) -> Status {
    use diesel::RunQueryDsl;
    use log::error;

    // if no text for the task was submitted, return an error
    if new_task_data.title.is_empty() {
        return Status::BadRequest;
    }

    // prepare the DTO for creating the new task
    let new_task = NewTask {
        title: new_task_data.title.clone(),
        owner: authenticated_user.id,
    };

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Status::InternalServerError;
        }
    };

    // add the DTO to the database
    let entries_added = diesel::insert_into(tasks::table)
        .values(&new_task)
        .execute(db_connection)
        .unwrap();

    // check if the task was added to the database
    if entries_added == 0 {
        return Status::InternalServerError;
    }
    Status::NoContent
}
