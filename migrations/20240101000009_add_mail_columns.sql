-- Add missing columns to mail table for paper style, font color, and item category

ALTER TABLE mail ADD COLUMN item_cat INTEGER DEFAULT 0;
ALTER TABLE mail ADD COLUMN paper INTEGER DEFAULT 0;
ALTER TABLE mail ADD COLUMN font_color INTEGER DEFAULT 1;
