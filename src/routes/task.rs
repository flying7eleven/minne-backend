use crate::fairings::MinneDatabaseConnection;
use crate::guards::AuthenticatedUser;
use crate::schema::tasks;
use chrono::NaiveDateTime;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub owner: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub done_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct SimplifiedTask {
    pub id: i32,
    pub title: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub title: String,
    pub owner: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Deserialize)]
pub struct NewTaskSuppliedData {
    /// The title for the new task.
    pub title: String,
    /// An optional time when the task was created. If this is not supplied, the current time will be used
    pub created_at: Option<NaiveDateTime>,
    /// An optional time when the task was last modified. If this is not supplied, the current time will be used
    pub updated_at: Option<NaiveDateTime>,
}

#[get("/task/list")]
pub async fn get_all_task_ids_from_user(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
) -> Result<Json<Vec<i32>>, Status> {
    use diesel::ExpressionMethods;
    use diesel::QueryDsl;
    use diesel::RunQueryDsl;
    use log::error;

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Err(Status::InternalServerError);
        }
    };

    // get all tasks of the authenticated user from the database
    let tasks = match tasks::table
        .filter(tasks::owner.eq(authenticated_user.id))
        .load::<Task>(db_connection)
    {
        Ok(tasks) => tasks,
        Err(error) => {
            error!(
                "Could not get all tasks of the user from the database. The error was: {}",
                error
            );
            return Err(Status::InternalServerError);
        }
    };

    // we do only need the ids of the tasks and nothing else
    let task_ids = tasks.into_iter().map(|task| task.id).collect();

    // return the fetch list of task ids
    return Ok(Json(task_ids));
}

#[post("/task", data = "<new_task_data>")]
pub async fn add_new_task(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    new_task_data: Json<NewTaskSuppliedData>,
) -> Result<Json<i32>, Status> {
    use diesel::RunQueryDsl;
    use log::error;

    // if no text for the task was submitted, return an error
    if new_task_data.title.is_empty() {
        return Err(Status::BadRequest);
    }

    // prepare the DTO for creating the new task
    let new_task = NewTask {
        title: new_task_data.title.clone(),
        owner: authenticated_user.id,
        created_at: new_task_data.created_at,
        updated_at: new_task_data.updated_at,
    };

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Err(Status::InternalServerError);
        }
    };

    // add the DTO to the database and get the generated id of the new task
    let maybe_task_id = diesel::insert_into(tasks::table)
        .values(&new_task)
        .returning(tasks::id)
        .get_result::<i32>(db_connection);

    // check if the task was added to the database and return an error if we failed to do so
    if maybe_task_id.is_err() {
        return Err(Status::InternalServerError);
    }

    // return the generated id for the corresponding task
    Ok(Json(maybe_task_id.unwrap()))
}

#[delete("/task/<task_id>")]
pub async fn delete_task(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    task_id: i32,
) -> Status {
    use crate::schema::tasks::{dsl::tasks, id};
    use diesel::ExpressionMethods;
    use diesel::{QueryDsl, RunQueryDsl};
    use log::{error, warn};

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

    // get the task DTO from the database based on the supplied task id
    let task = match tasks
        .filter(id.eq(task_id))
        .first::<Task>(&mut db_connection_pool.get().unwrap())
    {
        Ok(task) => task,
        Err(error) => {
            if error == diesel::NotFound {
                warn!(
                    "The user tried to delete a task with the id {} that does not exist.",
                    task_id
                );
                return Status::NotFound;
            }
            error!(
                "Could not get the task with the id {} from the database. The error was: {}",
                task_id, error
            );
            return Status::InternalServerError;
        }
    };

    // if the tasks does not belong to the authenticated user, return an error
    if task.owner != authenticated_user.id {
        return Status::Forbidden;
    }

    // delete the task from the database
    let entries_deleted = diesel::delete(tasks.filter(id.eq(task_id)))
        .execute(db_connection)
        .unwrap();

    // if the task was not deleted, return an error
    if entries_deleted == 0 {
        return Status::InternalServerError;
    }
    Status::NoContent
}

#[get("/task/<task_id>")]
pub async fn get_task(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    task_id: i32,
) -> Result<Json<SimplifiedTask>, Status> {
    use crate::schema::tasks::{dsl::tasks, id};
    use diesel::ExpressionMethods;
    use diesel::{QueryDsl, RunQueryDsl};
    use log::{error, warn};

    // get the task DTO from the database based on the supplied task id
    let task = match tasks
        .filter(id.eq(task_id))
        .first::<Task>(&mut db_connection_pool.get().unwrap())
    {
        Ok(task) => task,
        Err(error) => {
            if error == diesel::NotFound {
                warn!(
                    "The user tried to delete a task with the id {} that does not exist.",
                    task_id
                );
                return Err(Status::NotFound);
            }
            error!(
                "Could not get the task with the id {} from the database. The error was: {}",
                task_id, error
            );
            return Err(Status::InternalServerError);
        }
    };

    // if the tasks does not belong to the authenticated user, return an error
    if task.owner != authenticated_user.id {
        return Err(Status::Forbidden);
    }

    // convert the task DTO to a SimplifiedTask DTO
    let simplified_task = SimplifiedTask {
        id: task.id,
        title: task.title,
        created_at: task.created_at,
        updated_at: task.updated_at,
        done_at: task.done_at,
    };

    // return the simplified task DTO
    Ok(Json(simplified_task))
}
