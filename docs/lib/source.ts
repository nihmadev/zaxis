import { docs } from '../.source/server';
import { loader } from 'fumadocs-core/source';
import { createElement } from 'react';
import {
  BookOpen, Download, Terminal, FlaskConical, AppWindow, Rows3, Fingerprint,
  Keyboard, Timer, Palette, Shapes, Plug, Monitor, Braces, Gauge,
  List, CircleSlash, Wrench, Code,
  Blocks, PanelsTopLeft, Type, MousePointer2, SquareCheck, SlidersHorizontal, Minus,
  MoveVertical, Grid2X2, Table2, ListFilter, ToggleRight, BadgeAlert,
  Columns2, ListTree, Grip, ListCollapse, MessageSquare, ListOrdered, Image, ChevronsUpDown,
} from 'lucide-react';

const icons = {
  BookOpen, Download, Terminal, FlaskConical, AppWindow, Rows3, Fingerprint,
  Keyboard, Timer, Palette, Shapes, Plug, Monitor, Braces, Gauge,
  List, CircleSlash, Wrench, Code,
  Blocks, PanelsTopLeft, Type, MousePointer2, SquareCheck, SlidersHorizontal, Minus,
  MoveVertical, Grid2X2, Table2, ListFilter, ToggleRight, BadgeAlert,
  Columns2, ListTree, Grip, ListCollapse, MessageSquare, ListOrdered, Image, ChevronsUpDown,
};

export const source = loader({
  baseUrl: '/',
  source: docs.toFumadocsSource(),
  icon(name) {
    const Icon = icons[name as keyof typeof icons];
    return Icon ? createElement(Icon) : undefined;
  },
});
