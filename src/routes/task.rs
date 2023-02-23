use crate::fairings::MinneDatabaseConnection;
use crate::guards::AuthenticatedUser;
use crate::schema::tasks;
use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use serde::{Deserialize, Serialize};

#[derive(Queryable)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub owner: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct SimplifiedTask {
    pub id: i32,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub title: String,
    pub owner: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct NewTaskSuppliedData {
    /// The title for the new task.
    pub title: String,
    /// An optional time when the task was created. If this is not supplied, the current time will be used
    pub created_at: Option<DateTime<FixedOffset>>,
    /// An optional time when the task was last modified. If this is not supplied, the current time will be used
    pub updated_at: Option<DateTime<FixedOffset>>,
}

#[derive(Deserialize)]
pub struct TaskEditData {
    pub title: Option<String>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}

#[derive(Deserialize)]
pub struct UnknownTasksRequest {
    pub known_task_ids: Vec<i32>,
}

#[post("/task/filter", data = "<unknown_task_request>")]
pub async fn get_all_tasks_which_are_not_known(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    unknown_task_request: Json<UnknownTasksRequest>,
) -> Result<Json<Vec<i32>>, Status> {
    use diesel::ExpressionMethods;
    use diesel::QueryDsl;
    use diesel::RunQueryDsl;
    use log::{debug, error};

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

    // ensure we log how many tasks we found to be sure that the filtering works
    debug!(
        "Found {} tasks for the user {} before filtering them",
        authenticated_user.id,
        tasks.len()
    );

    // we do only need the ids of the tasks which are not already known and nothing else
    let task_ids: Vec<i32> = tasks
        .into_iter()
        .map(|task| task.id)
        .filter(|item| !unknown_task_request.known_task_ids.contains(item))
        .collect();

    // ensure we log how many tasks we have after filtering them to be sure that the filtering works
    debug!(
        "Found {} tasks for the user {} after filtering them",
        authenticated_user.id,
        task_ids.len()
    );

    // return the fetch list of task ids
    return Ok(Json(task_ids));
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

#[put("/task/<task_id>", data = "<task_change_data>")]
pub async fn edit_task(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    task_change_data: Json<TaskEditData>,
    task_id: i32,
) -> Status {
    use crate::diesel::ExpressionMethods;
    use crate::diesel::QueryDsl;
    use crate::diesel::RunQueryDsl;
    use crate::schema::tasks::dsl::{id, owner};
    use crate::schema::tasks::table;
    use log::error;

    // if non of the fields for the task were supplied, return an error
    if task_change_data.title.is_none() && task_change_data.updated_at.is_none() {
        return Status::BadRequest;
    }

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

    // return an error if the task with the supplied id does not exist or does not belong to the authenticated user
    if table
        .filter(id.eq(task_id))
        .filter(owner.eq(authenticated_user.id))
        .first::<Task>(db_connection)
        .is_err()
    {
        return Status::NotFound;
    }

    // if the title was supplied, update the task with the new title
    if let Some(new_title) = &task_change_data.title {
        if diesel::update(table.filter(id.eq(task_id)))
            .set(tasks::title.eq(new_title))
            .execute(db_connection)
            .is_err()
        {
            error!(
                "Could not update the title of the task with id {}.",
                task_id
            );
            return Status::InternalServerError;
        }
    }

    // if a updated_at time was supplied, update the task with the new updated_at time
    if let Some(new_updated_at) = &task_change_data.updated_at {
        if diesel::update(table.filter(id.eq(task_id)))
            .set(tasks::updated_at.eq(new_updated_at.with_timezone(&Utc)))
            .execute(db_connection)
            .is_err()
        {
            error!(
                "Could not update the updated_at time of the task with id {}.",
                task_id
            );
            return Status::InternalServerError;
        }
    }

    // if we reach this place, we executed all the changes successfully
    return Status::NoContent;
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
        created_at: new_task_data
            .created_at
            .map(|time| time.with_timezone(&Utc)),
        updated_at: new_task_data
            .updated_at
            .map(|time| time.with_timezone(&Utc)),
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
