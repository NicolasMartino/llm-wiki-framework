---
name: {{ self.name_yaml() }}
description: {{ self.description_yaml() }}
---

{% block title %}{% endblock %}

## Purpose

{{ self.purpose() }}

## Behavior

{{ self.behavior() }}

## Invocation

{{ self.invocation() }}{{ self.dispatcher_aliases() }}{{ self.notes_section() }}
