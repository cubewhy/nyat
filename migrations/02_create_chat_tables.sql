BEGIN;

CREATE TABLE IF NOT EXISTS chats(
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,

  type VARCHAR(20) NOT NULL CHECK (type IN ('private', 'group', 'channel')),
  created_at timestamptz NOT NULL DEFAULT current_timestamp
);

CREATE TABLE IF NOT EXISTS chat_participants (
  chat_id bigint NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
  user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role VARCHAR(20) CHECK (role IN ('member', 'admin', 'owner')),
  -- use bitmask to represent permission
  permission integer NOT NULL DEFAULT 0,
  added_at timestamptz NOT NULL DEFAULT current_timestamp,
  PRIMARY KEY (chat_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages(
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  chat_id bigint NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
  sender_id bigint NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  -- we will use jsonb later for the content field
  content text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT current_timestamp
);

COMMIT;
