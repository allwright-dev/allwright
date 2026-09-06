package dev.allwright.client;
/** Options for role matching. Null fields leave the corresponding constraint unset. */
public final class RoleOptions {
    Object name;
    public RoleOptions setName(Object value) { this.name = value; return this; }
    Boolean exact;
    public RoleOptions setExact(Boolean value) { this.exact = value; return this; }
    Boolean checked;
    public RoleOptions setChecked(Boolean value) { this.checked = value; return this; }
    Boolean disabled;
    public RoleOptions setDisabled(Boolean value) { this.disabled = value; return this; }
    Boolean expanded;
    public RoleOptions setExpanded(Boolean value) { this.expanded = value; return this; }
    Boolean includeHidden;
    public RoleOptions setIncludeHidden(Boolean value) { this.includeHidden = value; return this; }
    Integer level;
    public RoleOptions setLevel(Integer value) { this.level = value; return this; }
    Boolean pressed;
    public RoleOptions setPressed(Boolean value) { this.pressed = value; return this; }
    Boolean selected;
    public RoleOptions setSelected(Boolean value) { this.selected = value; return this; }
}
