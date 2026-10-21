#pragma once

#include "Memos.h"

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

// Multi-line text, as the board's body shows and edits it: its own line
// breaks ("\r\n", "\n" or "\r"), then lines wrapped at a width.
namespace ms::text_lines
{
	// A line's place in the text, without its line break.
	struct Line
	{
		size_t start;
		size_t length;
	};

	// The text's lines of at most `max` px by `width`. A wrapped line ends
	// where the next starts; a broken one ends before its line break.
	template <typename Width>
	std::vector<Line> layout(const std::string& text, int16_t max, Width width)
	{
		std::vector<Line> lines;
		size_t start = 0;

		while (true)
		{
			size_t end = text.find_first_of("\r\n", start);
			size_t length = (end == std::string::npos ? text.size() : end) - start;
			size_t at = start;

			for (const std::string& line : Memos::wrap(text.substr(start, length), max, width))
			{
				lines.push_back({ at, line.size() });
				at += line.size();
			}

			if (end == std::string::npos)
				break;

			start = end + (text.compare(end, 2, "\r\n") == 0 ? 2 : 1);
		}

		return lines;
	}

	// The line a position is on: a wrapped line's end is the next line's
	// start.
	inline size_t line_of(const std::vector<Line>& lines, size_t pos)
	{
		size_t line = 0;

		while (line + 1 < lines.size() && lines[line + 1].start <= pos)
			line++;

		return line;
	}

	// The first of `rows` lines shown out of `count` once `line` is among
	// them, moving as little as it can from `top`, and with no rows past
	// the last line left empty.
	inline size_t scroll_to(size_t top, size_t rows, size_t count, size_t line)
	{
		if (line < top)
			top = line;
		else if (line >= top + rows)
			top = line + 1 - rows;

		size_t last = count > rows ? count - rows : 0;

		return top < last ? top : last;
	}

	// A one-line edit's scroll, in px, so the 1 px caret at `caret` px
	// shows in `width` px of a text `total` px wide (0x4cbc03): past the
	// right edge the caret goes to the edge, `margin` px further in when
	// text follows it; past the left edge it goes `margin` px in, but no
	// further back than the start. A view narrower than the margin then
	// puts the caret back on its edge. Deleting never scrolls back.
	inline int16_t scroll_x(int16_t scroll, int16_t width, int16_t caret, int16_t total, int16_t margin)
	{
		int16_t right = caret + 1;

		if (right > scroll + width)
			scroll = right - width + (right < total ? margin : 0);

		if (caret < scroll)
			scroll = caret > margin ? caret - margin : 0;

		if (right > scroll + width)
			scroll = right - width;

		if (caret < scroll)
			scroll = caret;

		return scroll;
	}

	// One character left or right, taking "\r\n" as one.
	inline size_t step_left(const std::string& text, size_t pos)
	{
		if (pos == 0)
			return 0;

		return pos >= 2 && text.compare(pos - 2, 2, "\r\n") == 0 ? pos - 2 : pos - 1;
	}

	inline size_t step_right(const std::string& text, size_t pos)
	{
		if (pos >= text.size())
			return text.size();

		return text.compare(pos, 2, "\r\n") == 0 ? pos + 2 : pos + 1;
	}
}
