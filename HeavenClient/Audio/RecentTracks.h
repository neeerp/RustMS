#pragma once

#include <algorithm>
#include <cstddef>
#include <deque>
#include <string>
#include <vector>

namespace ms
{
	// The background tracks whose decoded sound is kept, newest first. A track decodes in whole
	// (a town's is some 30 MB), so only the last few are kept, enough that walking between two
	// maps doesn't decode again. v83 streams its one track and keeps none.
	class RecentTracks
	{
	public:
		explicit RecentTracks(size_t limit) : limit(limit) {}

		// Puts the track first and returns those no longer kept, oldest last.
		std::vector<std::string> use(const std::string &path)
		{
			auto iter = std::find(tracks.begin(), tracks.end(), path);

			if (iter != tracks.end())
				tracks.erase(iter);

			tracks.push_front(path);

			std::vector<std::string> dropped;

			while (tracks.size() > limit)
			{
				dropped.push_back(tracks.back());
				tracks.pop_back();
			}

			return dropped;
		}

		const std::deque<std::string> &kept() const
		{
			return tracks;
		}

	private:
		size_t limit;
		std::deque<std::string> tracks;
	};
}
