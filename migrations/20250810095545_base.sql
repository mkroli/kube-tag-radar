CREATE TABLE IF NOT EXISTS container (
    namespace TEXT NOT NULL,
    pod TEXT NOT NULL,
    container TEXT NOT NULL,
    image TEXT NOT NULL,
    image_id TEXT NOT NULL,
    latest_tag TEXT NOT NULL,
    latest_version_req TEXT NOT NULL,
    latest_version_regex TEXT NOT NULL,
    PRIMARY KEY(namespace, pod, container)
);

CREATE TABLE IF NOT EXISTS image (
    image TEXT NOT NULL,
    image_id TEXT NOT NULL,
    latest_tag TEXT NOT NULL,
    latest_version_req TEXT NOT NULL,
    latest_version_regex TEXT NOT NULL,
    resolved_image_id TEXT,
    latest_image_id TEXT,
    version TEXT,
    latest_version TEXT,
    last_checked DATETIME,
    PRIMARY KEY(image, image_id, latest_tag, latest_version_req, latest_version_regex)
);
