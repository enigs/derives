## Derives
Custom derives to help you manage drivers and database access via sqlx

### Required Libraries
- Third party libraries
    - `actix-web`
    - `sqlx`
- Internal libraries:
    - `ciphers`
    - `errors`
    - `nulls`
    - `responses`

### Encryption
Primarily used under drivers and for structs with `Cipher` struct under `ciphers::Cipher`.

**Example:**
```rust
use ciphers::Cipher;
use derives::Encryption;
use nulls::Null;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq)]
#[derive(Deserialize, Serialize, Encryption)]
#[serde(rename_all = "camelCase")]
pub struct Foo {
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[encryption(types = String, sanitize = "trim_slash")]
    pub bar: Null<Cipher>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[encryption(types = String, sanitize = "trim")]
    pub baz: Null<Cipher>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[encryption(errors = Vec<String>)]
    pub qux: Null<Cipher>,
}
```

The code above uses `Encryption` derive and enables `encryption` attribute derives to perform additional task and generate required codes and implementation for struct `Foo`
- It creates `.as_response()` implementation for struct `Foo` which is basically an actix web response type
- It generates a struct `FooForm` and `FooError`
    - Adding `errors` will overwrite `qux` type into `Vec<String>`. All errors defaults to type `String`
    - Adding `sanitize` will give `FooForm` `sanitize()` functionality which executes either `trim` or `trim_slash` (trims the last `/` within the text value)


### PostgreSQL
Primarily used to bridge sqlx connections. This will generate field names based on your struct and functions to parse through sqlx query.

**Example:**
```rust
#[derive(Default, Debug, Clone, PartialEq)]
#[derive(Deserialize, Serialize, PostgreSQL)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = String)]
    pub id: Null<String>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = i64)]
    pub cursor: Null<i64>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = String)]
    pub slug: Null<String>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = String)]
    pub name: Null<String>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = DateTime<Utc>)]
    pub created_at: Null<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Null::is_undefined")]
    #[psql(types = DateTime<Utc>)]
    pub updated_at: Null<DateTime<Utc>>,
}
```

The code above will create the following implementations for `Template`.
- `is_empty(&self) -> bool` - Checks if `Template` struct is empty (by referencing its default state).
- `to<T: From<Self>>(&self) -> T` - Allows `foo.to::<T>()` conversion if `T::from(Self)` is implemented.
- `to_json(&self) -> sqlx::types::Json<Self>` - Wraps self in sqlx's json wrapper.
- `$field() -> Option<T>` - Produces option wrapped type of that certain field.

It will also create a submodule of the following:
- `alias` - All aliased table + column names. Example format `foo.bar AS foo_bar` where foo is the *table* name and bar is the *column* name.
    - `all()` - Example: `foo::alias::all()` - Returns all aliased column names joined with comma as static string.
    - `$field_name()` - Example: `foo::alias::bar()` - Returns format for bar column as static string.
- `table` - Same implementation with alias except the format is `foo.bar`.
- `renamed`- Same implementation with alis except with the `foo_bar` formatting.
- `parsers` - Creates parsers that extract row data and converts it to its current struct.
    - `parse(row: &sqlx::postgres::PgRow) -> Self` - Example `foo::parsers::parse(result)` produces `Foo` struct.
    - `result(row: sqlx::Result<sqlx::postgres::PgRow>) -> actix_web::Result<Self>` - Produces `Result<Foo>`.
    - `relational(row: &sqlx::postgres::PgRow) -> Null<#node>` - Produces `Null<Foo>`.


### Enums
Derive macro that helps with trait implementation for enum types as sqlx String types. It helps with serde's serialization and deserialization and converts enum variants into SNAKE_CASE (uppercase) when read or saved from the database.

### IsEmpty
Appends `is_empty()` function that checks for `*self == Self::default()` value

### Jsonb
Appends both `is_empty()` and `to_json()` that wraps self instance into sqlx's jsonb