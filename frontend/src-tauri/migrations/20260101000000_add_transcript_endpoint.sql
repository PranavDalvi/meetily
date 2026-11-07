-- Migration: Add endpoint to transcript_settings table
ALTER TABLE transcript_settings ADD COLUMN endpoint TEXT;
