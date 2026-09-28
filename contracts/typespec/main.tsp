import "@typespec/json-schema";

using TypeSpec.JsonSchema;

namespace IndieBuild.Desktop;

@minLength(1)
@maxLength(256)
@pattern("^[a-z0-9][a-z0-9._-]*$")
scalar ServiceName extends string;

@minLength(1)
@maxLength(4096)
scalar CommandText extends string;

@maxLength(8192)
scalar CommandArgument extends string;

@maxItems(128)
model Arguments is Array<CommandArgument>;

@minLength(1)
@maxLength(4096)
scalar WorkingDirectory extends string;

@minLength(1)
@maxLength(256)
@pattern("^[A-Z_][A-Z0-9_]*$")
scalar EnvName extends string;

@maxLength(16384)
scalar EnvValue extends string;

@maxItems(128)
@uniqueItems
model EnvPassthrough is Array<EnvName>;

@maxProperties(128)
model Environment is Record<EnvValue>;

model ServiceSpec {
  name: ServiceName;
  command: CommandText;
  args?: Arguments = #[];
  working_dir?: WorkingDirectory;
  env_passthrough?: EnvPassthrough = #[];
  env?: Environment = #{};
  autostart?: boolean = false;
}

@maxItems(64)
model Services is Array<ServiceSpec>;

@minLength(1)
@maxLength(256)
scalar TunnelName extends string;

@minLength(1)
@maxLength(253)
scalar Hostname extends string;

@maxLength(2048)
@pattern("^http://(127\\.0\\.0\\.1|\\[::1\\]):[0-9]{1,5}/?$")
scalar LoopbackHttpOrigin extends string;

model TunnelSpec {
  name: TunnelName;
  hostname?: Hostname;
  service_url: LoopbackHttpOrigin;
  autostart?: boolean = false;
}

model UpdateSpec {
  command: CommandText;
  args?: Arguments = #[];
}

model DesktopManifest {
  version: 1;
  services: Services;
  tunnel?: TunnelSpec;
  update?: UpdateSpec;
}
