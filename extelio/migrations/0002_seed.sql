-- Rollen-Templates (6.5) und Default-Retention (14.1)
INSERT INTO roles (id,key,name,permissions,builtin,created_at) VALUES
 ('role-sysadmin','systemadministrator','Systemadministrator','["*"]',1,'1970-01-01T00:00:00Z'),
 ('role-telephony','telefonieadministrator','Telefonieadministrator','["telephony.*","routing.*","provisioning.*","device.*","number.*","trunk.*","health.read","audit.read"]',1,'1970-01-01T00:00:00Z'),
 ('role-supervisor','supervisor','Supervisor','["telephony.read","queue.*","people.read","health.read","call.read"]',1,'1970-01-01T00:00:00Z'),
 ('role-user','benutzer','Benutzer','["self.*","people.read","call.self"]',1,'1970-01-01T00:00:00Z'),
 ('role-readonly','nur_lesen','Nur Lesen','["*.read"]',1,'1970-01-01T00:00:00Z');

INSERT INTO retention_policies (key,purpose,days,updated_at) VALUES
 ('call_events','Betrieb und Fehleranalyse',90,'1970-01-01T00:00:00Z'),
 ('cdr','Abrechnung und Nachweis',180,'1970-01-01T00:00:00Z'),
 ('statistics','Aggregierte Auswertung',365,'1970-01-01T00:00:00Z'),
 ('security_audit','Sicherheitsnachweis',365,'1970-01-01T00:00:00Z'),
 ('system_logs','Betrieb',30,'1970-01-01T00:00:00Z'),
 ('voicemail','Kommunikation',60,'1970-01-01T00:00:00Z'),
 ('recordings','Dokumentation',30,'1970-01-01T00:00:00Z');

INSERT INTO provider_profiles (id,key,name,registrar,proxy,transport,codecs,number_format,capabilities,auth_capabilities,builtin,created_at) VALUES
 ('pp-generic-register','generic-register','Generic SIP (Registration)',NULL,NULL,'udp','["OPUS","PCMA","PCMU"]','e164','{"early_media":true,"clip_no_screening":false}','["register","digest"]',1,'1970-01-01T00:00:00Z'),
 ('pp-generic-static','generic-static','Generic SIP (Static IP)',NULL,NULL,'udp','["OPUS","PCMA","PCMU"]','e164','{"early_media":true,"clip_no_screening":true}','["ip_auth"]',1,'1970-01-01T00:00:00Z'),
 ('pp-generic-tls','generic-tls','Generic SIP (TLS/SRTP)',NULL,NULL,'tls','["OPUS","PCMA"]','e164','{"srtp":true,"early_media":true}','["register","digest"]',1,'1970-01-01T00:00:00Z');

INSERT INTO settings (key,value,updated_at) VALUES
 ('security.mode','"strict"','1970-01-01T00:00:00Z'),
 ('session.idle_timeout_minutes','30','1970-01-01T00:00:00Z'),
 ('session.absolute_timeout_hours','12','1970-01-01T00:00:00Z'),
 ('ha.integration_enabled','true','1970-01-01T00:00:00Z'),
 ('privacy.ha_personal_data','false','1970-01-01T00:00:00Z');
