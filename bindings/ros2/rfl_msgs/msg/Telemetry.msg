# rfl_msgs/msg/Telemetry — the standalone telemetry topic (spec/03
# TelemetryFeedback). Identical to the Execute action's feedback, published on a
# topic for consumers not tied to one goal's action lifecycle (the design-doc
# table maps telemetry to BOTH the action feedback and this topic).
string action_id                 # the action this sample belongs to
float64 t                        # timestamp on the spec/04 manifold timebase (s)
string realized_pose_json        # spec/02 Pose6DFloor ("" if none)
string wrench_json               # spec/04 WrenchFloor ("" if none)
string securing_force            # Force quantity ("" if none)
string station_error             # Length quantity ("" if none)
string tactile_json              # array of spec/04 readings ("[]" if none)
string events_json               # array of spec/04 ForceEvents ("[]" if none)
string fidelity_tier             # in effect this sample ("" if none)
string contact_geometry_json     # spec/05 contact_geometry ("" if none)
