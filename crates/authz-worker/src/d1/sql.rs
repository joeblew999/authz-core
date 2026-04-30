pub const INSERT_TUPLE: &str = "INSERT OR REPLACE INTO authz_tuple \
    (object_type, object_id, relation, subject_type, subject_id, condition, created_at) \
    VALUES (?, ?, ?, ?, ?, ?, ?)";

pub const DELETE_TUPLE: &str = "DELETE FROM authz_tuple \
    WHERE object_type = ? AND object_id = ? AND relation = ? \
      AND subject_type = ? AND subject_id = ?";

pub const SELECT_USER_TUPLE: &str = "SELECT object_type, object_id, relation, \
       subject_type, subject_id, condition \
    FROM authz_tuple \
    WHERE object_type = ? AND object_id = ? AND relation = ? \
      AND subject_type = ? AND subject_id = ? \
    LIMIT 1";

pub const SELECT_USERSET: &str = "SELECT object_type, object_id, relation, \
       subject_type, subject_id, condition \
    FROM authz_tuple \
    WHERE object_type = ? AND object_id = ? AND relation = ?";

pub const SELECT_STARTING_WITH_USER: &str = "SELECT object_type, object_id, relation, \
       subject_type, subject_id, condition \
    FROM authz_tuple \
    WHERE subject_type = ? AND subject_id = ?";
