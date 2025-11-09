CREATE TABLE IF NOT EXISTS users(
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  username text NOT NULL,
  password text NOT NULL,

  created_at timestamptz NOT NULL DEFAULT current_timestamp
);
