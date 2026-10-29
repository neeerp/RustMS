//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "TextLayout.h"

#include "Utf8.h"

namespace ms
{
	namespace
	{
		class LayoutBuilder
		{
		public:
			LayoutBuilder(const std::string &text, int16_t linespace,
						  const std::function<int16_t(char32_t, bool)> &advance, Text::Alignment alignment,
						  int16_t maxwidth, bool formatted, int16_t line_adj,
						  const std::vector<Point<int16_t>> &boxes);

			size_t add(size_t prev, size_t first, size_t last);

			Text::Layout finish(size_t first, size_t last);

		private:
			void add_word(size_t first, size_t last, Text::Font font, Color::Name color, bool bold);

			void add_line();

			// The advance of the character at `pos`: an inline box's width, and
			// nothing for the later bytes of a character.
			int16_t advance_of(size_t pos) const;

			const std::string &text;
			int16_t linespace;
			const std::function<int16_t(char32_t, bool)> &advance;

			Text::Alignment alignment;
			Text::Font fontid;
			Color::Name color;
			bool bold;
			// Inside a #c run: the next '#' ends it.
			bool highlight;
			int16_t maxwidth;
			bool formatted;

			int16_t ax;
			int16_t ay;

			std::vector<Text::Layout::Line> lines;
			std::vector<Text::Layout::Word> words;
			std::vector<int16_t> advances;
			int16_t width;
			int16_t endy;
			int16_t line_adj;

			// Inline boxes: their sizes by text position, the ones on the
			// current line, and where each was placed.
			std::map<size_t, Point<int16_t>> boxes;
			std::vector<std::pair<size_t, int16_t>> line_boxes;
			std::map<size_t, Point<int16_t>> box_positions;
			int16_t line_top;
		};

		LayoutBuilder::LayoutBuilder(const std::string &t, int16_t ls, const std::function<int16_t(char32_t, bool)> &adv,
									 Text::Alignment a, int16_t mw, bool fm, int16_t la,
									 const std::vector<Point<int16_t>> &b) : text(t), linespace(ls), advance(adv),
																			 alignment(a), maxwidth(mw), formatted(fm),
																			 line_adj(la)
		{
			fontid = Text::Font::NUM_FONTS;
			color = Color::Name::NUM_COLORS;
			bold = false;
			highlight = false;
			ax = 0;
			ay = linespace;
			width = 0;
			endy = 0;
			line_top = 0;

			size_t next_box = 0;

			for (size_t pos = 0; pos < text.size() && next_box < b.size(); pos++)
				if (text[pos] == Text::BOX)
					boxes[pos] = b[next_box++];

			if (maxwidth == 0)
				maxwidth = 800;
		}

		size_t LayoutBuilder::add(size_t prev, size_t first, size_t last)
		{
			if (first == last)
				return prev;

			Text::Font last_font = fontid;
			Color::Name last_color = color;
			bool last_bold = bold;
			size_t skip = 0;
			bool linebreak = false;

			if (formatted)
			{
				switch (text[first])
				{
					case '\\':
						if (first + 1 < last)
						{
							switch (text[first + 1])
							{
								case 'n':
									linebreak = true;
									break;
								case 'r':
									linebreak = ax > 0;
									break;
							}

							skip++;
						}

						skip++;
						break;
					case '#':
						if (highlight)
						{
							// As v83's tooltip text (0x8e4217): a '#' after #c
							// closes the run whatever follows, and only the
							// '#' is skipped, so "#c%s#'s" keeps its "'s".
							color = Color::Name::NUM_COLORS;
							highlight = false;
						}
						else if (first + 1 < last)
						{
							switch (text[first + 1])
							{
								case 'k':
									// Back to the text's own colour: dark grey in NPC
									// dialogue, black in quest descriptions.
									color = Color::Name::NUM_COLORS;
									break;
								case 'b':
									color = Color::Name::BLUE;
									break;
								case 'g':
									color = Color::Name::GREEN;
									break;
								case 'r':
									color = Color::Name::RED;
									break;
								case 'c':
									color = Color::Name::ORANGE;
									highlight = true;
									break;
								case 'd':
									color = Color::Name::SCRIPTPURPLE;
									break;
								case 'e':
									bold = true;
									break;
								case 'n':
									bold = false;
									break;
								case 'w':
									// v83's "show at once" toggle (FUN_0099dd54);
									// the NPC dialog acts on it.
									break;
							}

							skip++;
						}

						skip++;
						break;
				}
			}

			int16_t wordwidth = 0;

			if (!linebreak)
			{
				// Markup such as #k takes no room, so it can't push a word onto
				// the next line (leaving the space after it at the line's start).
				for (size_t i = first + skip; i < last; i++)
				{
					wordwidth += advance_of(i);

					if (wordwidth > maxwidth)
					{
						// Split before the character that doesn't fit, or after it
						// when it is the first: one character wider than the line
						// still gets a line of its own.
						size_t split = i == first ? Utf8::span(text, i, last) : i;

						if (split == last)
							break;

						prev = add(prev, first, split);
						return add(prev, split, last);
					}
				}
			}

			bool newword = skip > 0;
			// A word that doesn't fit an empty line stays on it.
			bool newline = linebreak || ax > 0 && ax + wordwidth > maxwidth;

			if (newword || newline)
				add_word(prev, first, last_font, last_color, last_bold);

			if (newline)
			{
				add_line();

				endy = ay;
				ax = 0;
				ay += linespace;

				if (lines.size() > 0)
					ay -= line_adj;

				line_top = ay - linespace;
			}

			for (size_t pos = first; pos < last; pos++)
			{
				char c = text[pos];

				advances.push_back(ax);

				if (pos < first + skip || newline && c == ' ')
					continue;

				auto box = boxes.find(pos);

				if (box != boxes.end())
				{
					// A box sits on the baseline, pushing the line down if it is taller.
					line_boxes.emplace_back(pos, ax);
					ay = std::max<int16_t>(ay, line_top + box->second.y());
				}

				ax += advance_of(pos);

				if (width < ax)
					width = ax;
			}

			if (newword || newline)
				return first + skip;
			else
				return prev;
		}

		Text::Layout LayoutBuilder::finish(size_t first, size_t last)
		{
			add_word(first, last, fontid, color, bold);
			add_line();

			advances.push_back(ax);

			std::vector<Point<int16_t>> positions;

			for (const auto &box : boxes)
			{
				auto placed = box_positions.find(box.first);
				positions.push_back(placed == box_positions.end() ? Point<int16_t>() : placed->second);
			}

			return Text::Layout(lines, advances, width, ay, ax, endy, positions);
		}

		int16_t LayoutBuilder::advance_of(size_t pos) const
		{
			auto box = boxes.find(pos);

			if (box != boxes.end())
				return box->second.x();

			char32_t codepoint;

			if (Utf8::decode(text, pos, codepoint) == 0)
				return 0;

			return advance(codepoint, bold);
		}

		void LayoutBuilder::add_word(size_t word_first, size_t word_last, Text::Font word_font,
									 Color::Name word_color, bool word_bold)
		{
			words.push_back({word_first, word_last, word_font, word_color, word_bold});
		}

		void LayoutBuilder::add_line()
		{
			int16_t line_x = 0;
			int16_t line_y = ay;

			switch (alignment)
			{
				case Text::Alignment::CENTER:
					line_x -= ax / 2;
					break;
				case Text::Alignment::RIGHT:
					line_x -= ax;
					break;
			}

			for (const auto &box : line_boxes)
				box_positions[box.first] = Point<int16_t>(line_x + box.second, line_y - boxes[box.first].y());

			lines.push_back({words, {line_x, line_y}, ax});
			words.clear();
			line_boxes.clear();
		}
	}

	Text::Layout layout_text(const std::string &text, int16_t linespace,
							 const std::function<int16_t(char32_t, bool)> &advance, Text::Alignment alignment,
							 int16_t maxwidth, bool formatted, int16_t line_adj,
							 const std::vector<Point<int16_t>> &boxes)
	{
		size_t length = text.length();

		if (length == 0)
			return Text::Layout();

		LayoutBuilder builder(text, linespace, advance, alignment, maxwidth, formatted, line_adj, boxes);

		const std::string separators = { ' ', '\\', '#', Text::BOX };

		size_t first = 0;
		size_t offset = 0;

		while (offset < length)
		{
			size_t last = text.find_first_of(separators, offset + 1);

			if (last == std::string::npos)
				last = length;

			first = builder.add(first, offset, last);
			offset = last;
		}

		return builder.finish(first, offset);
	}

	Text::Layout::Layout(const std::vector<Layout::Line> &l, const std::vector<int16_t> &a, int16_t w, int16_t h,
						 int16_t ex, int16_t ey, const std::vector<Point<int16_t>> &b) : lines(l), advances(a),
																						 dimensions(w, h),
																						 endoffset(ex, ey), boxes(b)
	{}

	Text::Layout::Layout() : Layout(std::vector<Layout::Line>(), std::vector<int16_t>(), 0, 0, 0, 0)
	{}

	int16_t Text::Layout::width() const
	{
		return dimensions.x();
	}

	int16_t Text::Layout::height() const
	{
		return dimensions.y();
	}

	int16_t Text::Layout::advance(size_t index) const
	{
		return index < advances.size() ? advances[index] : 0;
	}

	Point<int16_t> Text::Layout::get_dimensions() const
	{
		return dimensions;
	}

	Point<int16_t> Text::Layout::get_endoffset() const
	{
		return endoffset;
	}

	Point<int16_t> Text::Layout::box(size_t index) const
	{
		return index < boxes.size() ? boxes[index] : Point<int16_t>();
	}

	size_t Text::Layout::index_at(Point<int16_t> point) const
	{
		// Below the baseline, a line keeps the descenders of its letters.
		constexpr int16_t DESCENT = 4;

		int16_t top = 0;

		for (const Line &line : lines)
		{
			int16_t bottom = line.position.y() + DESCENT;

			if (point.y() >= top && point.y() < bottom)
			{
				int16_t x = point.x() - line.position.x();

				if (line.words.empty() || x < 0 || x >= line.width)
					return std::string::npos;

				size_t found = std::string::npos;

				for (size_t pos = line.words.front().first; pos < line.words.back().last; pos++)
					if (advance(pos) <= x)
						found = pos;

				return found;
			}

			top = bottom;
		}

		return std::string::npos;
	}

	Text::Layout::iterator Text::Layout::begin() const
	{
		return lines.begin();
	}

	Text::Layout::iterator Text::Layout::end() const
	{
		return lines.end();
	}
}
