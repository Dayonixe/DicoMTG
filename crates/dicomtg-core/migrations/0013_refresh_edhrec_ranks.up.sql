-- Reload reference data once after upgrading to the corrected rank import.
DELETE FROM reference_update_state WHERE singleton = 1;
