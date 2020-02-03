## CHANGELOG:
* 0.11.0
    * Cockpits and Processors do not accept multiple items with the same name
    * `TelemetryTransmitterSync` is deprecated since the `TelemetryTransmitter` is sync
* 0.10.15
    * DataDisplay - Reset value is also initial value
* 0.10.14
    * `DataDisplay` has a default value to switch back to.
* 0.10.13
    * Added instrument `DataDisplay` which simply displays an observation (for some time).
* 0.10.12
    * Add instrument to collect data from jemalloc. Requires feature `jemalloc-ctl`.
    * Impl `From<Snapshot>` for `ItemKind.
* 0.10.11
    * Transfer the meter from metrics 0.2.3 here since the crate in that repository has changed to a whole new project [commit](https://github.com/chridou/metrix/commit/693e86e839b8870bcbfae93f1416d094ee2e88a6)
* 0.10.10
    * Added a LabelFilter that can be used to filter for labels [commit](https://github.com/chridou/metrix/commit/75b142d6a791dc3b0654985f2f7333b60014b004)
* 0.10.9
    * Tests and documentation for gauge [commit](https://github.com/chridou/metrix/commit/4b9939f657f1dfd59dfd2b55491df8eec904f77e)
    * Deprecates set_instrument methods in favour of add_instrument [commit](https://github.com/chridou/metrix/commit/2cbd31b89788c0b3b386ae9b83b3136d49d52128)
    * removed instrument getters on panel
* 0.10.8
    * BUGFIX: Gauge tracked with non-equidistant intervals
* 0.10.7
    * allow remapping of updates
    * filter for valid labels by predicates
* 0.10.6
    * Gauge has sliding window for tracking peak values
    * added more builder like methods
* 0.10.5
    * Adapters shouldn't be instruments
* 0.10.4
    * Fix bug in Gauge which prevents updates
* 0.10.2
    * Gauge can display measured time
* 0.10.1
    * Removed some unnecessary `Clone` bounds
* 0.10.0 **Breaking Changes
    * Builder like API for instruments and panels
    * Instruments can handle observations via adapter
    * Instruments and panels can accept multiple labels
    * Gauge has different strategies for updating values
    * `ValueScaling` removed. Instruments have a `TimeUnit` for displaying values
    * Observations have typed values in an enum `ObservedValue`
    * Deprecated magic values `INCR`, `DECR`, `TRUE`, `FALSE`. There are variants for these observation now.
* 0.9.24
    * Processors check whether they are disconnected
* 0.9.23
    * `FoundItem` should also have a find method
* 0.9.22
    * Exit the processor if all drivers are gone
* 0.9.21
    * Exit the driver if all drivers are gone
* 0.9.20
    * Do not log an error if the channel is empty
* 0.9.18
    * Improved find item to implement display on `ItemKind`
* 0.9.17
    * Deprecated `IncDecGauge`
    * Gauge supports increment and decrement
* 0.9.16
    * Added an instrument `IncDecGauge` which can only be increased or decreased
* 0.9.15
    * Fix bug in NonOccurence tracker which had a logic error
* 0.9.14
    * Instruments that can show the inverted state have more options to change the name for the inverted value
* 0.9.13
    * All instruments that display booleans can show there inverted state
* 0.9.12
    * Flag can show its inverted value
* 0.9.11
    * Value of Flag can be `None`
* 0.9.10
    * Added a boolean flag
* 0.9.9
    * Inactivity tracking for panels
* 0.9.8
    * Fixed set values for gauges
* 0.9.7 use `<=` and `>=` for detecting peaks and bottoms
    * Added peak and bottom tracking to `Gauge`
* 0.9.6
    * Added peak and bottom tracking to `Gauge`
* 0.9.5
    * Added peak tracking to `Gauge`
* 0.9.2
    * updated crossbeam
* 0.9.2
    * measure number of instrumets updated per second
* 0.9.1
    * `TelemetryDriver` now supports a processing strategy
* 0.9.0
    * `TelemetryDriver` has a builder
    * `TelemetryDriver` is immutable
    * Snapshots are calculated in the background thread
    * Snapshots can be queried with as a `Future`
* 0.8.3
    * Use crossbeam channels
* 0.8.1
    * Fixed bug always reporting all meter rate if 1 munite was enabled
* 0.8.0
    * Meters only have 1 minute rate enabled by default
    * Histograms can track inactivity and reset themselves
    * Breaking changes: Moved some traits into other packages
