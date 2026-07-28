## REMOVED Requirements

### Requirement: Single complete text view
**Reason**: Phase One replaces the production text-only window contract with the authored graphical application shell.

**Migration**: Preserve the complete deterministic text projection as read-only workspace diagnostic content nested in `GraphicalShellProjection`; migrate production window consumers to the graphical projection while retaining diagnostic and verification access to text.

#### Scenario: Production window requests its view
- **WHEN** the production window renders an accepted application generation
- **THEN** it consumes the immutable graphical shell rather than a top-level text-only contract, while the complete text projection remains available as nested read-only diagnostic content
