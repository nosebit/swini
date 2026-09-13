# Croft Clerk Module

Domain staff component responsible for Croft registration and discovery queries
across the Ranch:

- **[`mod.rs`](./mod.rs)**: Domain [`Clerk`](./mod.rs) managing registration and
  inquiries against Barn consensus storage.
- **[`api.rs`](./api.rs)**: gRPC service handler [`Api`](./api.rs) implementing
  the tonic-generated `CroftApi` trait.
