- macro for making models
  - should set Option<T> fields to 
    - `#[serde(skip_serializing_if = "Option::is_none")]`
    - `#[builder(setter(strip_option), default)]`
  - should set Vec<T> fields to 
    - `#[serde(skip_serializing_if = "Vec::is_empty")]`
    - `#[builder(default)]`
  - should set HashMap<T,X> fields to 
    - `#[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]`
    - `#[builder(default)]`
  - should have *option* for `deleted` field
    - auto-impl for `Type.is_deleted()`
  - should have *option* for `active` field
    - auto-impl for `Type.is_active()` (detects `is_deleted()`)
  - should implement a `Type::builder()` impl fn if builder is used
- solidify access system
  - create a `RoleHasPermission` trait where Role (global, company) implement the 
  `can()` function
  - create a `CanAccess` trait, which objects with roles (users, members) implement
  and just requires retuning a set of roles implementing `RoleHasPermission`

