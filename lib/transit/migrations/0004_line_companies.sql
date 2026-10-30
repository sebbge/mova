-- line company, learned from buses in service
CREATE TABLE line_companies (
    region   TEXT NOT NULL,
    provider TEXT NOT NULL,
    line     TEXT NOT NULL,
    company  TEXT NOT NULL,
    seen_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (region, provider, line, company)
);
