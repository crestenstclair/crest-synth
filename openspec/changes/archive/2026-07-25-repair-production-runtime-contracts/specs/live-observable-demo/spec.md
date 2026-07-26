## ADDED Requirements

### Requirement: Live device health remains visible
The physical live demo SHALL monitor the bounded runtime device-status path for the lifetime of its stream. A post-start device failure SHALL stop the live runtime from presenting a healthy window, preserve the typed failure outside the callback, and return it as an application-visible error without emitting successful completion or selecting a silent, headless, or alternate-device fallback.

#### Scenario: Physical stream fails during the live scene
- **WHEN** the stream starts successfully and later reports a device failure before live completion
- **THEN** control observes the exact typed failure, ends the unhealthy window lifetime, suppresses a successful live report, and returns the error

#### Scenario: Physical stream remains healthy
- **WHEN** no runtime failure is reported
- **THEN** device-status polling does not mutate the scene, projections, audio, checkpoints, or user-controlled final window lifetime
