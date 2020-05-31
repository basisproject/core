- solidify access system
  - create a `RoleHasPermission` trait where Role (global, company) implement the 
  `can()` function
  - create a `CanAccess` trait, which objects with roles (users, members) implement
  and just requires retuning a set of roles implementing `RoleHasPermission`
- when processing EconomicEvents, moved Costs can be higher than the source Costs,
  resulting in "zeroing out" the source costs.
  - should this be allowed?
    - no

