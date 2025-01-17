use crate::{AppProtocol, ChartType, IpVersion, ReportType, TransProtocol};

#[derive(Debug, Clone)]
/// Messages types that permit to react to application interactions/subscriptions
pub enum Message {
    /// Every 5 seconds
    TickInit,
    /// Every 1 second
    TickRun,
    /// Select adapter
    AdapterSelection(String),
    /// Select IP filter
    IpVersionSelection(IpVersion),
    /// Select transport filter
    TransportProtocolSelection(TransProtocol),
    /// Select application filter
    AppProtocolSelection(AppProtocol),
    /// Select chart type to be displayed
    ChartSelection(ChartType),
    /// Select report type to be displayed
    ReportSelection(ReportType),
    /// Open Sniffnet's complete textual report
    OpenReport,
    /// Open Sniffnet's GitHub page
    OpenGithub,
    /// Start sniffing packets
    Start,
    /// Stop sniffing process and return to initial page
    Reset,
    /// Change application style (day or night)
    Style,
}