-- SampleFarm Database Initialization Script
-- PostgreSQL Version
-- Supports idempotent execution

-- Create database (if not exists)
-- Note: In production environments, databases are typically created separately by DBAs or deployment scripts
-- CREATE DATABASE samplefarm 
--     WITH 
--     OWNER = postgres
--     ENCODING = 'UTF8'
--     LC_COLLATE = 'en_US.UTF-8'
--     LC_CTYPE = 'en_US.UTF-8'
--     TABLESPACE = pg_default
--     CONNECTION LIMIT = -1;

-- Switch to target database
-- \c samplefarm;

-- Clean up existing types (if they exist)
DROP TYPE IF EXISTS sample_type_enum CASCADE;
DROP TYPE IF EXISTS analyzer_type CASCADE;
DROP TYPE IF EXISTS master_task_status_enum CASCADE;
DROP TYPE IF EXISTS sub_task_status_enum CASCADE;

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Create custom types
CREATE TYPE sample_type_enum AS ENUM ('Benign', 'Malicious');

-- Analyzer type enum (includes CFG)
CREATE TYPE analyzer_type AS ENUM ('CAPE', 'CFG');

-- Master task status enum
CREATE TYPE master_task_status_enum AS ENUM (
    'pending',     -- Pending execution
    'running',     -- Currently running
    'paused',      -- Paused
    'completed',   -- Completed
    'failed',      -- Failed
    'cancelled'    -- Cancelled
);

-- Sub-task status enum
CREATE TYPE sub_task_status_enum AS ENUM (
    'pending',      -- Waiting for submission
    'submitting',   -- Currently submitting
    'submitted',    -- Submitted
    'analyzing',    -- Under analysis
    'paused',       -- Paused
    'completed',    -- Completed
    'failed',       -- Failed
    'cancelled'     -- Cancelled
);

-- =====================================================
-- Initialization Data
-- =====================================================