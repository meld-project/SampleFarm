-- SampleFarm Database Complete Deployment Script
-- PostgreSQL Version
-- Supports idempotent execution
-- 
-- Usage:
-- psql -U postgres -d samplefarm -f deploy.sql

-- Execute initialization script
\i database/init.sql

-- Execute table structure script
\i database/schema.sql

-- Deployment completion messages
\echo 'Database deployment completed!'
\echo 'All tables, indexes, constraints, and triggers have been created.'
\echo 'Script supports idempotent execution.'