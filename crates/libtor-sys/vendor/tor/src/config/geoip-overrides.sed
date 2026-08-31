# replace region 3221225728,3221226495 with the
# two regions below of the same country,
# excluding 192.0.2.0/24
s/3221225728,3221226495,\(..\)/3221225728,3221225983,\1\n3221226240,3221226495,\1/g

# in case the above region expands (low - 256),
# replace region 3221225472,3221226495 with the
# two regions below of the same country,
# excluding 192.0.2.0/24
s/3221225472,3221226495,\(..\)/3221225472,3221225983,\1\n3221226240,3221226495,\1/g

# remove 192.0.2.0/24 as a whole region
/3221225984,3221226239,../d

# remove 198.51.100.0/24 as a whole region
/3325256704,3325256959,../d

# in case the above region expands (low - 256),
# replace region 3325256448,3325256959 with
# 3325256448,3325256703 of the same country,
# excluding 198.51.100.0/24
s/3325256448,3325256959,\(..\)/3325256448,3325256703,\1/g

# in case the above region merges with the previous one,
# replace region 3325255680,3325256959 with
# 3325255680,3325256703 of the same country,
# excluding 198.51.100.0/24
s/3325255680,3325256959,\(..\)/3325255680,3325256703,\1/g

# remove 203.0.113.0/24, which has no room to expand
/3405803776,3405804031,../d

# remove 2001:db8::/32, which has no room to expand
/2001:db8::,2001:db8:ffff:ffff:ffff:ffff:ffff:ffff,../d
