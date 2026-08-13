// @ts-nocheck
import ChatHeader from "./ChatHeader";
import MessageStream from "./MessageStream";
import MessageComposer from "./MessageComposer";

export default function ChatCanvas(props: any) {
  return <div className="glass-card" style={{ padding: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}><ChatHeader {...props} /><MessageStream {...props} /><MessageComposer {...props} /></div>;
}
