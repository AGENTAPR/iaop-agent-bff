-- Allow login transactions to record an alternate (allowlisted) redirect URI,
-- e.g. when the BFF login flow is reached through the developer portal proxy.
ALTER TABLE bff_login_transactions
    ADD COLUMN IF NOT EXISTS redirect_uri TEXT;
