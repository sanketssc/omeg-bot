```
// if command
//     .user
//     .has_role(&ctx.http, 696775407764242523, 69677885235304827)
//     .await
//     .unwrap()
// {
//     println!("User has role ");
// } else {
//     println!("User does not have role");
// }

// let guild_id = GuildId::new(696775407764242523);
// let role_id = RoleId::new(696778852353048627);

// println!(
//     "roles: {:?}",
//     guild_id
//         .roles(&ctx.http)
//         .await
//         .unwrap()
//         .get(&role_id)
//         .unwrap()
//         .name
// );```