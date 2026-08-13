// @ts-nocheck
export default function MessageContextMenu(props: any) {
  const { contextMenu, hubMessages, startEdit, deleteMessage } = props;
  return <>
      {contextMenu && (() => {
        const msg = hubMessages.find(m => m.id === contextMenu.messageId);
        if (!msg) return null;
        return (
          <div
            className="glass-card"
            onClick={e => e.stopPropagation()}
            onContextMenu={e => e.preventDefault()}
            style={{
              position: "fixed",
              top: contextMenu.y,
              left: contextMenu.x,
              zIndex: 1000,
              padding: "0.35rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.15rem",
              minWidth: "140px",
              boxShadow: "0 8px 24px rgba(0,0,0,0.4)"
            }}
          >
            <button
              onClick={() => startEdit(msg)}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-main)",
                textAlign: "left",
                padding: "0.45rem 0.6rem",
                borderRadius: "6px",
                fontSize: "0.85rem",
                cursor: "pointer"
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "rgba(255,255,255,0.08)")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              ✏️ Edit
            </button>
            <button
              onClick={() => deleteMessage(msg.id)}
              style={{
                background: "transparent",
                border: "none",
                color: "#f87171",
                textAlign: "left",
                padding: "0.45rem 0.6rem",
                borderRadius: "6px",
                fontSize: "0.85rem",
                cursor: "pointer"
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "rgba(248,113,113,0.1)")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              🗑️ Delete
            </button>
          </div>
        );
      })()}
  </>;
}
