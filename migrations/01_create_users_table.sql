CREATE TABLE users(
  id bigint GENERATED ALWAYS AS IDENTITY,
  username text,
  password text,

  created_at timestamptz default current_timestamp
);
