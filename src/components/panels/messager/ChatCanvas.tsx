// @ts-nocheck
import ChatHeader from "./ChatHeader";
import MessageStream from "./MessageStream";
import MessageComposer from "./MessageComposer";
import HarnessSessionStrip from "../harness/HarnessSessionStrip";
import HarnessDeliveryBanner from "../harness/HarnessDeliveryBanner";

export default function ChatCanvas(props: any) {
  return (
    <div className="glass-card" style={{ padding: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <ChatHeader {...props} />
      <HarnessSessionStrip sessions={props.harnessSessions ?? []} workspace={props.workspacePath ?? ""} />
      <HarnessDeliveryBanner
        notices={props.deliveryNotices ?? []}
        onRetry={props.onRetryDelivery}
        onDismiss={props.onDismissDelivery}
        retryingHarness={props.retryingHarness ?? null}
      />
      <MessageStream {...props} />
      <MessageComposer {...props} />
    </div>
  );
}
