-- SampleFarm Core Table Structure
-- PostgreSQL Version
-- Supports idempotent execution

-- Clean up existing objects (in dependency order)
DROP TRIGGER IF EXISTS update_samples_updated_at ON samples;
DROP TRIGGER IF EXISTS update_master_tasks_updated_at ON master_tasks;
DROP TRIGGER IF EXISTS update_cape_analysis_results_updated_at ON cape_analysis_results;
DROP FUNCTION IF EXISTS update_updated_at_column() CASCADE;
DROP TABLE IF EXISTS samples CASCADE;

-- Samples table
CREATE TABLE samples (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id UUID REFERENCES samples(id) ON DELETE CASCADE, -- Parent sample ID (ZIP package), NULL indicates top-level file
    is_container BOOLEAN NOT NULL DEFAULT FALSE, -- Whether it's a container file (ZIP, etc.)
    file_path_in_zip VARCHAR(500), -- File path within ZIP archive
    file_name VARCHAR(255) NOT NULL, -- File name
    file_hash_md5 VARCHAR(32) NOT NULL, -- MD5 hash
    file_hash_sha1 VARCHAR(40) NOT NULL, -- SHA1 hash
    file_hash_sha256 VARCHAR(64) NOT NULL, -- SHA256 hash
    file_size BIGINT NOT NULL, -- File size in bytes
    file_type VARCHAR(100), -- MIME type
    file_extension VARCHAR(50), -- File extension
    storage_path VARCHAR(500) NOT NULL, -- Storage path (unified use of storage_path instead of minio_path)
    sample_type sample_type_enum NOT NULL, -- Sample type
    labels JSONB, -- Label array (unified use of labels instead of tags)
    source TEXT, -- Source
    custom_metadata JSONB, -- Custom metadata (unified use of custom_metadata instead of remarks)
    zip_password TEXT, -- Encrypted ZIP password (only valid for container files)
    run_filename VARCHAR(255), -- Run filename (only valid for ZIP packages)
    has_custom_metadata BOOLEAN NOT NULL DEFAULT FALSE, -- Whether sub-files have custom metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Samples table indexes
CREATE INDEX IF NOT EXISTS idx_samples_parent_id ON samples(parent_id);
CREATE INDEX IF NOT EXISTS idx_samples_file_hash_md5 ON samples(file_hash_md5);
CREATE INDEX IF NOT EXISTS idx_samples_file_hash_sha1 ON samples(file_hash_sha1);
CREATE INDEX IF NOT EXISTS idx_samples_file_hash_sha256 ON samples(file_hash_sha256);
CREATE INDEX IF NOT EXISTS idx_samples_is_container ON samples(is_container);
CREATE INDEX IF NOT EXISTS idx_samples_sample_type ON samples(sample_type);
CREATE INDEX IF NOT EXISTS idx_samples_created_at ON samples(created_at);
CREATE INDEX IF NOT EXISTS idx_samples_labels ON samples USING GIN(labels);
CREATE INDEX IF NOT EXISTS idx_samples_file_name ON samples USING GIN(file_name gin_trgm_ops);

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Add updated_at trigger for samples table
CREATE TRIGGER update_samples_updated_at 
    BEFORE UPDATE ON samples 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Add constraints (idempotent handling)
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_file_size_positive') THEN
        ALTER TABLE samples ADD CONSTRAINT chk_file_size_positive CHECK (file_size > 0);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_md5_format') THEN
        ALTER TABLE samples ADD CONSTRAINT chk_md5_format CHECK (char_length(file_hash_md5) = 32);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_sha1_format') THEN
        ALTER TABLE samples ADD CONSTRAINT chk_sha1_format CHECK (char_length(file_hash_sha1) = 40);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_sha256_format') THEN
        ALTER TABLE samples ADD CONSTRAINT chk_sha256_format CHECK (char_length(file_hash_sha256) = 64);
    END IF;
END $$;

-- Add unique constraint to prevent duplicates (only applies to top-level files)
CREATE UNIQUE INDEX IF NOT EXISTS idx_samples_md5_unique ON samples(file_hash_md5) WHERE parent_id IS NULL;

-- Table and column comments
COMMENT ON TABLE samples IS 'Sample file storage table, supports parent-child relationships for ZIP packages';
COMMENT ON COLUMN samples.parent_id IS 'Parent sample ID, NULL indicates top-level file, non-NULL indicates file extracted from ZIP package';
COMMENT ON COLUMN samples.is_container IS 'Whether it is a container file (like ZIP), true indicates archive, false indicates regular file';
COMMENT ON COLUMN samples.file_path_in_zip IS 'Relative path of file within ZIP package';
COMMENT ON COLUMN samples.has_custom_metadata IS 'Indicates whether sub-files have user-defined custom metadata';
COMMENT ON COLUMN samples.storage_path IS 'File path in storage system';
COMMENT ON COLUMN samples.labels IS 'File label array, supports GIN index queries';
COMMENT ON COLUMN samples.zip_password IS 'ZIP file password, only valid for container files';
COMMENT ON COLUMN samples.run_filename IS 'Main executable filename in ZIP package';

-- =====================================================
-- Task Management Related Tables
-- =====================================================

-- Clean up existing task-related objects
DROP TABLE IF EXISTS cape_analysis_results CASCADE;
DROP TABLE IF EXISTS sub_tasks CASCADE;
DROP TABLE IF EXISTS master_tasks CASCADE;
DROP TABLE IF EXISTS cape_instances CASCADE;
DROP TABLE IF EXISTS cfg_instances CASCADE;
DROP TABLE IF EXISTS cfg_analysis_results CASCADE;

-- CAPE instances table
CREATE TABLE cape_instances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,              -- CAPE instance name
    base_url VARCHAR(255) NOT NULL,                 -- CAPE API address
    description TEXT,                                -- Description
    enabled BOOLEAN NOT NULL DEFAULT true,          -- Whether enabled
    timeout_seconds INTEGER NOT NULL DEFAULT 300,   -- Timeout duration
    max_concurrent_tasks INTEGER NOT NULL DEFAULT 5, -- Maximum concurrent tasks
    health_check_interval INTEGER NOT NULL DEFAULT 60, -- Health check interval (seconds)
    status VARCHAR(20) NOT NULL DEFAULT 'unknown',  -- Health status: healthy, unhealthy, unknown
    last_health_check TIMESTAMPTZ,                  -- Last health check time
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT chk_cape_timeout_positive CHECK (timeout_seconds > 0),
    CONSTRAINT chk_cape_concurrent_positive CHECK (max_concurrent_tasks > 0),
    CONSTRAINT chk_cape_health_interval_positive CHECK (health_check_interval > 0),
    CONSTRAINT chk_cape_status_valid CHECK (status IN ('healthy', 'unhealthy', 'unknown'))
);

-- CAPE instances table indexes
CREATE INDEX IF NOT EXISTS idx_cape_instances_enabled ON cape_instances(enabled);
CREATE INDEX IF NOT EXISTS idx_cape_instances_status ON cape_instances(status);
CREATE INDEX IF NOT EXISTS idx_cape_instances_last_health_check ON cape_instances(last_health_check);

-- CAPE instances table comments
COMMENT ON TABLE cape_instances IS 'CAPE instance configuration table, supports multi-CAPE instance management';
COMMENT ON COLUMN cape_instances.name IS 'CAPE instance display name';
COMMENT ON COLUMN cape_instances.base_url IS 'CAPE API base URL';
COMMENT ON COLUMN cape_instances.enabled IS 'Whether this instance is enabled';
COMMENT ON COLUMN cape_instances.status IS 'Instance health status';
COMMENT ON COLUMN cape_instances.last_health_check IS 'Last health check time';

-- CFG实例表（与CAPE结构对齐）
CREATE TABLE cfg_instances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    base_url VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    timeout_seconds INTEGER NOT NULL DEFAULT 300,
    max_concurrent_tasks INTEGER NOT NULL DEFAULT 2,
    health_check_interval INTEGER NOT NULL DEFAULT 60,
    status VARCHAR(20) NOT NULL DEFAULT 'unknown',
    last_health_check TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_cfg_timeout_positive CHECK (timeout_seconds > 0),
    CONSTRAINT chk_cfg_concurrent_positive CHECK (max_concurrent_tasks > 0),
    CONSTRAINT chk_cfg_health_interval_positive CHECK (health_check_interval > 0),
    CONSTRAINT chk_cfg_status_valid CHECK (status IN ('healthy', 'unhealthy', 'unknown'))
);

CREATE INDEX IF NOT EXISTS idx_cfg_instances_enabled ON cfg_instances(enabled);
CREATE INDEX IF NOT EXISTS idx_cfg_instances_status ON cfg_instances(status);
CREATE INDEX IF NOT EXISTS idx_cfg_instances_last_health_check ON cfg_instances(last_health_check);

COMMENT ON TABLE cfg_instances IS 'CFG实例配置表，支持多CFG实例管理';
COMMENT ON COLUMN cfg_instances.name IS 'CFG实例显示名称';
COMMENT ON COLUMN cfg_instances.base_url IS 'CFG API基础URL';
COMMENT ON COLUMN cfg_instances.enabled IS '是否启用该实例';
COMMENT ON COLUMN cfg_instances.status IS '实例健康状态';
COMMENT ON COLUMN cfg_instances.last_health_check IS '最后一次健康检查时间';

-- 主任务表
CREATE TABLE master_tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_name VARCHAR(255) NOT NULL,
    analyzer_type analyzer_type NOT NULL,        -- 分析器类型
    task_type VARCHAR(50) NOT NULL DEFAULT 'batch',  -- 任务类型（batch/single等）
    total_samples INTEGER NOT NULL DEFAULT 0,
    completed_samples INTEGER NOT NULL DEFAULT 0,
    failed_samples INTEGER NOT NULL DEFAULT 0,
    status master_task_status_enum NOT NULL DEFAULT 'pending',
    progress INTEGER NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    error_message TEXT,
    result_summary JSONB,
    sample_filter JSONB,  -- 保存创建任务时的样本筛选条件
    paused_at TIMESTAMPTZ,     -- 暂停时间
    pause_reason TEXT,         -- 暂停原因
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 主任务表索引
CREATE INDEX IF NOT EXISTS idx_master_tasks_status ON master_tasks(status);
CREATE INDEX IF NOT EXISTS idx_master_tasks_analyzer_type ON master_tasks(analyzer_type);
CREATE INDEX IF NOT EXISTS idx_master_tasks_created_at ON master_tasks(created_at);

-- 子任务表
CREATE TABLE sub_tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    master_task_id UUID NOT NULL REFERENCES master_tasks(id) ON DELETE CASCADE,
    sample_id UUID NOT NULL REFERENCES samples(id),
    analyzer_type analyzer_type NOT NULL,        -- 分析器类型（冗余存储方便查询）
    cape_instance_id UUID REFERENCES cape_instances(id), -- CAPE实例ID（可选，NULL表示使用默认实例）
    cfg_instance_id UUID REFERENCES cfg_instances(id),   -- CFG实例ID（可选，NULL表示使用默认实例）
    external_task_id VARCHAR(100),               -- 外部系统任务ID（如CAPE task_id）
    status sub_task_status_enum NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    parameters JSONB,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_sub_tasks_master_sample UNIQUE (master_task_id, sample_id)
);

-- 子任务表索引
CREATE INDEX IF NOT EXISTS idx_sub_tasks_master_task_id ON sub_tasks(master_task_id);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_sample_id ON sub_tasks(sample_id);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_status ON sub_tasks(status);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_cape_instance_id ON sub_tasks(cape_instance_id);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_cfg_instance_id ON sub_tasks(cfg_instance_id);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_external_task_id ON sub_tasks(external_task_id);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_updated_at ON sub_tasks(updated_at);
CREATE INDEX IF NOT EXISTS idx_sub_tasks_analyzer_status_updated ON sub_tasks(analyzer_type, status, updated_at);

-- CAPE分析结果表（精简版本，仅保留关键指标+完整报告JSONB）
CREATE TABLE cape_analysis_results (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sub_task_id UUID NOT NULL REFERENCES sub_tasks(id) ON DELETE CASCADE,
    sample_id UUID NOT NULL REFERENCES samples(id),
    cape_task_id INTEGER NOT NULL,              -- CAPE系统中的任务ID
    
    -- 基础信息
    analysis_started_at TIMESTAMPTZ,
    analysis_completed_at TIMESTAMPTZ,
    analysis_duration INTEGER,                   -- 分析耗时（秒）
    
    -- 分析结果摘要（关键指标）
    score DECIMAL(3,1) CHECK (score >= 0 AND score <= 10),  -- 恶意评分 (0-10)
    severity VARCHAR(20) CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    verdict VARCHAR(20) CHECK (verdict IN ('clean', 'suspicious', 'malicious')),
    
    -- 检测信息（JSONB格式）
    signatures JSONB,                           -- 命中的特征签名
    behavior_summary JSONB,                     -- 行为摘要
    
    -- 完整报告（所有细节都从这里提取）
    full_report JSONB,                          -- CAPE完整JSON报告（净化后用于查询/展示）
    full_report_raw BYTEA,                      -- 原始报告字节（可选，用于审计/追溯，可能包含不可存入JSONB的字符）
    report_summary TEXT,                        -- 报告摘要文本
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- CAPE分析结果表索引
CREATE INDEX IF NOT EXISTS idx_cape_results_sub_task_id ON cape_analysis_results(sub_task_id);
CREATE INDEX IF NOT EXISTS idx_cape_results_sample_id ON cape_analysis_results(sample_id);
CREATE INDEX IF NOT EXISTS idx_cape_results_cape_task_id ON cape_analysis_results(cape_task_id);
CREATE INDEX IF NOT EXISTS idx_cape_results_score ON cape_analysis_results(score);
CREATE INDEX IF NOT EXISTS idx_cape_results_verdict ON cape_analysis_results(verdict);
CREATE INDEX IF NOT EXISTS idx_cape_results_created_at ON cape_analysis_results(created_at);

-- 为任务表添加更新时间触发器
CREATE TRIGGER update_cape_instances_updated_at 
    BEFORE UPDATE ON cape_instances 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_master_tasks_updated_at 
    BEFORE UPDATE ON master_tasks 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sub_tasks_updated_at 
    BEFORE UPDATE ON sub_tasks 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_cape_analysis_results_updated_at 
    BEFORE UPDATE ON cape_analysis_results 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- 任务相关表注释
COMMENT ON TABLE master_tasks IS '主任务表，管理批量分析任务';
COMMENT ON TABLE sub_tasks IS '子任务表，每个样本的具体分析任务';
COMMENT ON TABLE cape_analysis_results IS 'CAPE沙箱分析结果存储表';
-- CFG分析结果表（简化版）
CREATE TABLE cfg_analysis_results (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sub_task_id UUID NOT NULL REFERENCES sub_tasks(id) ON DELETE CASCADE,
    sample_id UUID NOT NULL REFERENCES samples(id),
    message TEXT,
    result_files JSONB,
    full_report JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cfg_results_sub_task_id ON cfg_analysis_results(sub_task_id);
CREATE INDEX IF NOT EXISTS idx_cfg_results_sample_id ON cfg_analysis_results(sample_id);
CREATE INDEX IF NOT EXISTS idx_cfg_results_created_at ON cfg_analysis_results(created_at);

CREATE TRIGGER update_cfg_analysis_results_updated_at 
    BEFORE UPDATE ON cfg_analysis_results 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON COLUMN master_tasks.analyzer_type IS '使用的分析器类型';
COMMENT ON COLUMN master_tasks.sample_filter IS '创建任务时使用的样本筛选条件';
COMMENT ON COLUMN master_tasks.paused_at IS '任务暂停时间，用于审计记录';
COMMENT ON COLUMN master_tasks.pause_reason IS '任务暂停原因';
COMMENT ON COLUMN sub_tasks.external_task_id IS '外部分析系统的任务ID';
COMMENT ON COLUMN sub_tasks.updated_at IS '子任务最后更新时间，用于僵死任务检测和启动恢复功能';
COMMENT ON COLUMN cape_analysis_results.score IS 'CAPE恶意评分，0-10分';
COMMENT ON COLUMN cape_analysis_results.verdict IS '分析判定：clean(干净)/suspicious(可疑)/malicious(恶意)';
COMMENT ON COLUMN cape_analysis_results.full_report IS 'CAPE返回的完整JSON格式报告（净化后用于查询/展示）';
COMMENT ON COLUMN cape_analysis_results.full_report_raw IS '原始报告字节数据（可能含\u0000等字符，不适合直接存JSONB）';

-- ================================
-- CAPE性能统计表（已弃用：默认不创建，可按需启用）
-- ================================
/*
CREATE TABLE IF NOT EXISTS cape_performance_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_size BIGINT NOT NULL,
    submit_duration INTEGER, -- 提交耗时（秒）
    analysis_duration INTEGER, -- 分析耗时（秒）
    total_duration INTEGER, -- 总耗时（秒）
    throughput_mbps DOUBLE PRECISION, -- 吞吐量 MB/s
    status_check_count INTEGER NOT NULL DEFAULT 0, -- 状态检查次数
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE cape_performance_stats IS 'CAPE分析性能统计表';
COMMENT ON COLUMN cape_performance_stats.file_size IS '文件大小（字节）';
COMMENT ON COLUMN cape_performance_stats.submit_duration IS '文件提交耗时（秒）';
COMMENT ON COLUMN cape_performance_stats.analysis_duration IS '分析执行耗时（秒）';
COMMENT ON COLUMN cape_performance_stats.total_duration IS '总处理耗时（秒）';
COMMENT ON COLUMN cape_performance_stats.throughput_mbps IS '处理吞吐量（MB/s）';
COMMENT ON COLUMN cape_performance_stats.status_check_count IS '状态检查次数';

-- 性能统计索引
CREATE INDEX IF NOT EXISTS idx_cape_perf_stats_created_at ON cape_performance_stats(created_at);
CREATE INDEX IF NOT EXISTS idx_cape_perf_stats_file_size ON cape_performance_stats(file_size);
*/

-- ================================
-- CAPE任务状态快照表（已弃用：默认不创建，可按需启用）
-- ================================
/*
CREATE TABLE IF NOT EXISTS cape_task_status_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sub_task_id UUID UNIQUE NOT NULL REFERENCES sub_tasks(id) ON DELETE CASCADE,
    cape_instance_id UUID NOT NULL REFERENCES cape_instances(id) ON DELETE CASCADE,
    cape_task_id INT NOT NULL,
    status TEXT NOT NULL,
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 快照表索引
CREATE INDEX IF NOT EXISTS idx_cape_task_snapshots_instance_task
    ON cape_task_status_snapshots (cape_instance_id, cape_task_id);
CREATE INDEX IF NOT EXISTS idx_cape_task_snapshots_status
    ON cape_task_status_snapshots (status);
CREATE INDEX IF NOT EXISTS idx_cape_task_snapshots_updated_at
    ON cape_task_status_snapshots (updated_at);

-- 添加更新时间触发器
CREATE TRIGGER update_cape_task_status_snapshots_updated_at 
    BEFORE UPDATE ON cape_task_status_snapshots 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE cape_task_status_snapshots IS 'CAPE任务状态快照表，存储从list接口获取的运行时状态信息';
COMMENT ON COLUMN cape_task_status_snapshots.sub_task_id IS '对应的子任务ID，与内部任务系统关联';
COMMENT ON COLUMN cape_task_status_snapshots.cape_task_id IS 'CAPE系统中的任务ID';
COMMENT ON COLUMN cape_task_status_snapshots.status IS 'CAPE返回的原始状态字符串';
COMMENT ON COLUMN cape_task_status_snapshots.snapshot IS 'CAPE list接口返回的完整任务条目JSON数据';
*/

-- =====================================================
-- 默认实例数据（放在表结构之后，确保表存在）
-- =====================================================

-- 默认 CAPE 实例（幂等）
INSERT INTO cape_instances (id, name, base_url, description, enabled, timeout_seconds, max_concurrent_tasks)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    '默认CAPE实例',
    'http://192.168.2.186:8000/apiv2',
    '从config.toml迁移的默认CAPE实例',
    true,
    300,
    5
) ON CONFLICT (name) DO UPDATE SET
    base_url = EXCLUDED.base_url,
    description = EXCLUDED.description,
    enabled = EXCLUDED.enabled,
    timeout_seconds = EXCLUDED.timeout_seconds,
    max_concurrent_tasks = EXCLUDED.max_concurrent_tasks,
    updated_at = CURRENT_TIMESTAMP;

-- 默认 CFG 实例（幂等）
INSERT INTO cfg_instances (id, name, base_url, description, enabled, timeout_seconds, max_concurrent_tasks)
VALUES (
    '00000000-0000-0000-0000-000000000101'::uuid,
    '默认CFG实例',
    'http://localhost:17777',
    'CFG 分析默认实例（示例）',
    true,
    300,
    2
) ON CONFLICT (name) DO UPDATE SET
    base_url = EXCLUDED.base_url,
    description = EXCLUDED.description,
    enabled = EXCLUDED.enabled,
    timeout_seconds = EXCLUDED.timeout_seconds,
    max_concurrent_tasks = EXCLUDED.max_concurrent_tasks,
    updated_at = CURRENT_TIMESTAMP;